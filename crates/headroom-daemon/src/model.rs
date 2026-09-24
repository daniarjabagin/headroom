use std::collections::{BTreeMap, BTreeSet, HashMap};

use headroom_core::account::AccountId;
use headroom_core::provider::ProviderError;
use headroom_core::quota::{LimitsSnapshot, LimitsSource, WindowId};
use headroom_core::usage::UsageSummary;
use jiff::Timestamp;
use rusqlite::Connection;

use crate::dismissed::DismissedHomes;
use crate::error::StorageError;
use crate::home::UsageHome;
use crate::settings::Settings;
use crate::storage::accounts::{self, AccountRecord};
use crate::storage::lapses::{self, Lapse};
use crate::storage::{dismissed, settings, snapshots};
use crate::update::AvailableUpdate;

#[derive(Debug, Clone, Default)]
pub struct Model {
    pub settings: Settings,
    pub accounts: Vec<AccountRecord>,
    pub dismissed: DismissedHomes,
    pub snapshots: HashMap<AccountId, SnapshotEntry>,
    pub runtime: HashMap<AccountId, AccountRuntime>,
    pub usage_homes: BTreeSet<UsageHome>,
    pub usage: BTreeMap<UsageHome, UsageSummary>,
    pub update: Option<AvailableUpdate>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SnapshotEntry {
    pub snapshot: LimitsSnapshot,
    pub origin: SnapshotOrigin,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapshotOrigin {
    Cache,
    Refreshed,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct AccountRuntime {
    pub refreshing: bool,
    pub last_attempt: Option<Timestamp>,
    pub failure: Option<RefreshFailure>,
    pub failures: u32,
    pub hold_until: Option<Timestamp>,
    pub next_refresh_at: Option<Timestamp>,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RefreshFailure {
    #[error(transparent)]
    Provider(#[from] ProviderError),
    #[error("the provider did not answer in time")]
    Timeout,
    #[error("no provider is registered for this account")]
    NoProvider,
}

impl RefreshFailure {
    #[must_use]
    pub fn is_signed_out(&self) -> bool {
        matches!(
            self,
            RefreshFailure::Provider(ProviderError::NotSignedIn | ProviderError::SignInExpired)
        )
    }

    #[must_use]
    pub fn is_no_subscription(&self) -> bool {
        matches!(
            self,
            RefreshFailure::Provider(ProviderError::NoSubscription { .. })
        )
    }

    #[must_use]
    pub fn is_rate_limited(&self) -> bool {
        matches!(
            self,
            RefreshFailure::Provider(ProviderError::RateLimited { .. })
        )
    }

    #[must_use]
    pub fn is_network(&self) -> bool {
        matches!(self, RefreshFailure::Provider(ProviderError::Network(_)))
    }
}

impl SnapshotEntry {
    #[must_use]
    pub fn data_time(&self) -> Timestamp {
        match self.snapshot.source {
            LimitsSource::Live => self.snapshot.fetched_at,
            LimitsSource::LocalLog { observed_at } => observed_at,
        }
    }
}

impl Model {
    pub fn load(conn: &Connection) -> Result<Model, StorageError> {
        let snapshots = snapshots::load_all(conn)?
            .into_iter()
            .map(|(id, snapshot)| {
                let origin = SnapshotOrigin::Cache;
                (id, SnapshotEntry { snapshot, origin })
            })
            .collect();
        Ok(Model {
            settings: settings::load(conn)?,
            accounts: accounts::load_all(conn)?,
            dismissed: dismissed::load_all(conn)?,
            snapshots,
            runtime: lapsed_runtime(lapses::load_all(conn)?),
            usage_homes: BTreeSet::new(),
            usage: BTreeMap::new(),
            update: None,
        })
    }

    #[must_use]
    pub fn account(&self, id: &AccountId) -> Option<&AccountRecord> {
        self.active_accounts().find(|a| a.id() == id)
    }

    #[must_use]
    pub fn stored_account(&self, id: &AccountId) -> Option<&AccountRecord> {
        self.accounts.iter().find(|a| a.id() == id)
    }

    pub fn active_accounts(&self) -> impl Iterator<Item = &AccountRecord> {
        self.accounts
            .iter()
            .filter(|a| !a.gone && !self.dismissed.hides(&a.reference))
    }

    pub fn runtime_mut(&mut self, id: &AccountId) -> &mut AccountRuntime {
        self.runtime.entry(id.clone()).or_default()
    }

    pub fn record_success(&mut self, id: &AccountId, snapshot: LimitsSnapshot, now: Timestamp) {
        let origin = SnapshotOrigin::Refreshed;
        self.snapshots
            .insert(id.clone(), SnapshotEntry { snapshot, origin });
        let runtime = self.runtime_mut(id);
        runtime.last_attempt = Some(now);
        runtime.failure = None;
        runtime.failures = 0;
        runtime.hold_until = None;
    }

    pub fn record_failure(
        &mut self,
        id: &AccountId,
        failure: RefreshFailure,
        now: Timestamp,
        hold_until: Option<Timestamp>,
    ) {
        if failure.is_no_subscription() {
            self.snapshots.remove(id);
        }
        let runtime = self.runtime_mut(id);
        runtime.last_attempt = Some(now);
        runtime.failure = Some(failure);
        runtime.failures = runtime.failures.saturating_add(1);
        runtime.hold_until = hold_until;
    }
}

fn lapsed_runtime(lapses: Vec<Lapse>) -> HashMap<AccountId, AccountRuntime> {
    lapses
        .into_iter()
        .map(|lapse| {
            let error = ProviderError::NoSubscription {
                detail: lapse.detail,
            };
            let runtime = AccountRuntime {
                failure: Some(RefreshFailure::Provider(error)),
                ..AccountRuntime::default()
            };
            (lapse.account, runtime)
        })
        .collect()
}

#[must_use]
pub fn window_key(id: &WindowId) -> String {
    match id {
        WindowId::Session => "session".to_owned(),
        WindowId::Weekly => "weekly".to_owned(),
        WindowId::Model(name) => format!("model:{name}"),
        WindowId::Other(name) => format!("other:{name}"),
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::storage::Storage;
    use crate::testing::{CLAUDE, CODEX};
    use crate::testing::{session, snapshot, ts};

    const NOW_TEXT: &str = "2026-09-23T10:00:00Z";

    #[test]
    fn window_keys_are_flat_strings() {
        assert_eq!(window_key(&WindowId::Session), "session");
        assert_eq!(window_key(&WindowId::Weekly), "weekly");
        assert_eq!(window_key(&WindowId::Model("opus".into())), "model:opus");
        assert_eq!(window_key(&WindowId::Other("x".into())), "other:x");
    }

    #[test]
    fn failures_accumulate_until_success() {
        let mut model = Model::default();
        let id = AccountId(format!("{CODEX}:a"));
        let now = ts("2026-09-23T10:00:00Z");
        model.record_failure(&id, RefreshFailure::Timeout, now, None);
        model.record_failure(&id, RefreshFailure::Timeout, now, Some(now));
        assert_eq!(model.runtime[&id].failures, 2);
        assert_eq!(model.runtime[&id].hold_until, Some(now));
        let fresh = snapshot(
            vec![session(1.0, "2026-09-23T12:00:00Z")],
            "2026-09-23T10:00:00Z",
        );
        model.record_success(&id, fresh, now);
        assert_eq!(model.runtime[&id].failures, 0);
        assert_eq!(model.runtime[&id].failure, None);
        assert_eq!(model.snapshots[&id].origin, SnapshotOrigin::Refreshed);
    }

    #[test]
    fn signed_out_failures_are_recognised() {
        assert!(RefreshFailure::Provider(ProviderError::SignInExpired).is_signed_out());
        assert!(RefreshFailure::Provider(ProviderError::NotSignedIn).is_signed_out());
        assert!(!RefreshFailure::Provider(ProviderError::ApiKeyOnly).is_signed_out());
        assert!(!RefreshFailure::Timeout.is_signed_out());
    }

    #[test]
    fn no_subscription_failure_drops_the_snapshot() {
        let mut model = Model::default();
        let id = AccountId(format!("{CLAUDE}:a"));
        let now = ts("2026-09-23T10:00:00Z");
        let fresh = snapshot(vec![session(1.0, "2026-09-23T12:00:00Z")], NOW_TEXT);
        model.record_success(&id, fresh, now);
        let lapsed = RefreshFailure::Provider(ProviderError::NoSubscription {
            detail: "none".into(),
        });
        assert!(lapsed.is_no_subscription());
        model.record_failure(&id, lapsed, now, None);
        assert!(!model.snapshots.contains_key(&id));
        model.record_failure(&id, RefreshFailure::Timeout, now, None);
        assert!(!RefreshFailure::Timeout.is_no_subscription());
    }

    #[test]
    fn stored_lapses_seed_the_runtime_on_load() {
        let storage = Storage::open_in_memory().unwrap();
        let id = AccountId("codex:work".into());
        let model = storage
            .blocking(|conn| {
                lapses::record(conn, &id, "No active ChatGPT subscription.")?;
                Model::load(conn)
            })
            .unwrap();
        let failure = model.runtime[&id].failure.clone();
        assert_eq!(
            failure,
            Some(RefreshFailure::Provider(ProviderError::NoSubscription {
                detail: "No active ChatGPT subscription.".into()
            }))
        );
    }

    #[test]
    fn only_network_failures_count_as_network() {
        assert!(RefreshFailure::Provider(ProviderError::Network("down".into())).is_network());
        assert!(!RefreshFailure::Timeout.is_network());
        assert!(!RefreshFailure::Provider(ProviderError::SignInExpired).is_network());
    }

    #[test]
    fn local_log_snapshots_are_dated_by_observation() {
        let mut local = snapshot(Vec::new(), "2026-09-23T10:00:00Z");
        local.source = LimitsSource::LocalLog {
            observed_at: ts("2026-09-23T08:00:00Z"),
        };
        let entry = SnapshotEntry {
            snapshot: local,
            origin: SnapshotOrigin::Refreshed,
        };
        assert_eq!(entry.data_time(), ts("2026-09-23T08:00:00Z"));
    }
}
