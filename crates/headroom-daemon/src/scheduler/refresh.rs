use headroom_core::account::AccountRef;
use headroom_core::provider::ProviderError;
use headroom_core::quota::LimitsSnapshot;
use jiff::{SignedDuration, Timestamp};

use super::policy::{self, FETCH_TIMEOUT};
use crate::core::Core;
use crate::model::RefreshFailure;
use crate::notify::alerts::{LapseReview, Review};
use crate::notify::text::Locale;
use crate::storage::{accounts, lapses, samples, snapshots};

pub async fn refresh_account(core: &Core, account: &AccountRef) -> SignedDuration {
    begin_refresh(core, account);
    let fetched = fetch(core, account).await;
    if !is_current(core, account) {
        return abandon_refresh(core, account);
    }
    let now = core.clock.now();
    let delay = match fetched {
        Ok(snapshot) => {
            persist(core, account, &snapshot, now).await;
            core.alerts.renewed(&account.id);
            record_success(core, account, snapshot, now)
        }
        Err(failure) => {
            if let RefreshFailure::Provider(ProviderError::NoSubscription { detail }) = &failure {
                persist_lapse(core, account, detail.clone()).await;
            }
            record_failure(core, account, failure, now)
        }
    };
    review_alerts(core, account, now).await;
    finish_refresh(core, account, now.checked_add(delay).ok());
    delay
}

fn min_poll(core: &Core, account: &AccountRef) -> Option<SignedDuration> {
    core.catalog.min_poll_interval(&account.provider)
}

fn is_current(core: &Core, account: &AccountRef) -> bool {
    core.model()
        .active_accounts()
        .any(|record| &record.reference == account)
}

fn abandon_refresh(core: &Core, account: &AccountRef) -> SignedDuration {
    let mut model = core.model();
    if model.account(&account.id).is_none() {
        model.runtime_mut(&account.id).refreshing = false;
    }
    model.settings.refresh_interval()
}

async fn fetch(core: &Core, account: &AccountRef) -> Result<LimitsSnapshot, RefreshFailure> {
    let provider = core
        .provider(&account.provider)
        .ok_or(RefreshFailure::NoProvider)?;
    let limit = std::time::Duration::try_from(FETCH_TIMEOUT).unwrap_or_default();
    match tokio::time::timeout(limit, provider.fetch_limits(account)).await {
        Ok(result) => Ok(result?),
        Err(_) => Err(RefreshFailure::Timeout),
    }
}

async fn persist(core: &Core, account: &AccountRef, snapshot: &LimitsSnapshot, now: Timestamp) {
    let id = account.id.clone();
    let stored = snapshot.clone();
    let result = core
        .storage
        .run(move |conn| {
            snapshots::save(conn, &id, &stored)?;
            samples::record(conn, &id, &stored, now)?;
            lapses::clear(conn, &id)?;
            let identity = &stored.identity;
            accounts::set_identity(
                conn,
                &id,
                identity.email.as_deref(),
                identity.plan.as_deref(),
            )
        })
        .await;
    if let Err(error) = result {
        tracing::warn!(account = %account.id, %error, "could not store limits snapshot");
    }
}

async fn persist_lapse(core: &Core, account: &AccountRef, detail: String) {
    let id = account.id.clone();
    let result = core
        .storage
        .run(move |conn| {
            snapshots::delete(conn, &id)?;
            lapses::record(conn, &id, &detail)
        })
        .await;
    if let Err(error) = result {
        tracing::warn!(account = %account.id, %error, "could not store subscription lapse");
    }
}

fn record_success(
    core: &Core,
    account: &AccountRef,
    snapshot: LimitsSnapshot,
    now: Timestamp,
) -> SignedDuration {
    let mut model = core.model();
    if let Some(record) = model.accounts.iter_mut().find(|a| a.id() == &account.id) {
        record.email.clone_from(&snapshot.identity.email);
        record.plan.clone_from(&snapshot.identity.plan);
    }
    model.record_success(&account.id, snapshot, now);
    let live = model.account_is_live(account, now);
    let interval = policy::provider_interval(&model.settings, live, min_poll(core, account));
    policy::next_delay(Ok(()), 0, interval, core.random.unit())
        .max(policy::provider_floor(min_poll(core, account)))
}

fn record_failure(
    core: &Core,
    account: &AccountRef,
    failure: RefreshFailure,
    now: Timestamp,
) -> SignedDuration {
    tracing::info!(account = %account.id, error = %failure, "refresh failed");
    let mut model = core.model();
    let interval = model.settings.refresh_interval();
    let streak = policy::failure_streak(model.runtime.get(&account.id), &failure);
    let delay = policy::next_delay(Err(&failure), streak, interval, core.random.unit())
        .max(policy::provider_floor(min_poll(core, account)));
    let hold_until = policy::holds_soft_refresh(&failure)
        .then(|| now.checked_add(delay).ok())
        .flatten();
    model.record_failure(&account.id, failure, now, hold_until);
    delay
}

async fn review_alerts(core: &Core, account: &AccountRef, now: Timestamp) {
    let (record, snapshot, settings, lapsed, history, signal) = {
        let model = core.model();
        let record = model.account(&account.id).cloned();
        let snapshot = model.snapshots.get(&account.id).map(|e| e.snapshot.clone());
        let lapsed = model
            .runtime
            .get(&account.id)
            .and_then(|runtime| runtime.failure.as_ref())
            .is_some_and(RefreshFailure::is_no_subscription);
        let history = model.history.shared(&account.id);
        let signal = model.activity_signal(account, now);
        (
            record,
            snapshot,
            model.settings.clone(),
            lapsed,
            history,
            signal,
        )
    };
    let Some(record) = record else {
        return;
    };
    let locale = Locale::resolve(settings.display.language, core.system_locale);
    let provider_name = core.catalog.display_name(&record.reference.provider);
    let result = match snapshot {
        _ if lapsed => {
            let lapse = LapseReview {
                account: &record,
                provider_name,
                settings: &settings.notifications,
                locale,
                now,
                tz: &core.tz,
            };
            core.alerts.review_lapse(&lapse).await
        }
        Some(snapshot) => {
            let review = Review {
                account: &record,
                provider_name,
                snapshot: &snapshot,
                history: history.as_deref(),
                signal,
                settings: settings.notifications,
                display: &settings.display,
                locale,
                now,
                tz: &core.tz,
            };
            core.alerts.review(&review).await
        }
        None => Ok(()),
    };
    if let Err(error) = result {
        tracing::warn!(account = %account.id, %error, "could not store notification state");
    }
}

fn begin_refresh(core: &Core, account: &AccountRef) {
    let mut model = core.model();
    let runtime = model.runtime_mut(&account.id);
    runtime.refreshing = true;
    runtime.next_refresh_at = None;
    drop(model);
    core.mark_changed();
}

fn finish_refresh(core: &Core, account: &AccountRef, next_refresh_at: Option<Timestamp>) {
    let mut model = core.model();
    let runtime = model.runtime_mut(&account.id);
    runtime.refreshing = false;
    runtime.next_refresh_at = next_refresh_at;
    drop(model);
    core.mark_changed();
}
