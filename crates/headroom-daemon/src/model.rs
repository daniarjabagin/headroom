use std::collections::{BTreeMap, BTreeSet, HashMap};

use headroom_core::account::AccountId;
use headroom_core::provider::ProviderError;
use headroom_core::quota::{LimitsSnapshot, LimitsSource, WindowId};
use headroom_core::usage::UsageSummary;
use jiff::Timestamp;
use rusqlite::Connection;

use crate::error::StorageError;
use crate::home::UsageHome;
use crate::settings::Settings;
use crate::storage::accounts::{self, AccountRecord};
use crate::storage::{settings, snapshots};

#[derive(Debug, Clone, Default)]
pub struct Model {
    pub settings: Settings,
    pub accounts: Vec<AccountRecord>,
    pub snapshots: HashMap<AccountId, SnapshotEntry>,
    pub runtime: HashMap<AccountId, AccountRuntime>,
    pub usage_homes: BTreeSet<UsageHome>,
    pub usage: BTreeMap<UsageHome, UsageSummary>,
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
            snapshots,
            runtime: HashMap::new(),
            usage_homes: BTreeSet::new(),
            usage: BTreeMap::new(),
        })
    }

    #[must_use]
    pub fn account(&self, id: &AccountId) -> Option<&AccountRecord> {
        self.accounts.iter().find(|a| a.id() == id && !a.gone)
    }

    pub fn active_accounts(&self) -> impl Iterator<Item = &AccountRecord> {
        self.accounts.iter().filter(|a| !a.gone)
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
        let runtime = self.runtime_mut(id);
        runtime.last_attempt = Some(now);
        runtime.failure = Some(failure);
        runtime.failures = runtime.failures.saturating_add(1);
        runtime.hold_until = hold_until;
    }
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
    use headroom_core::account::ProviderKind;

    use super::*;
    use crate::testing::{session, snapshot, ts};

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
        let id = AccountId(format!("{}:a", ProviderKind::Codex));
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
