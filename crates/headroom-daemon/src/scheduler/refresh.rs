use headroom_core::account::AccountRef;
use headroom_core::quota::LimitsSnapshot;
use jiff::{SignedDuration, Timestamp};

use super::policy::{self, FETCH_TIMEOUT};
use crate::core::Core;
use crate::model::RefreshFailure;
use crate::notify::alerts::Review;
use crate::storage::{accounts, snapshots};

pub async fn refresh_account(core: &Core, account: &AccountRef) -> SignedDuration {
    begin_refresh(core, account);
    let fetched = fetch(core, account).await;
    let now = core.clock.now();
    let delay = match fetched {
        Ok(snapshot) => {
            persist(core, account, &snapshot).await;
            record_success(core, account, snapshot, now)
        }
        Err(failure) => record_failure(core, account, failure, now),
    };
    review_alerts(core, account, now).await;
    finish_refresh(core, account, now.checked_add(delay).ok());
    delay
}

async fn fetch(core: &Core, account: &AccountRef) -> Result<LimitsSnapshot, RefreshFailure> {
    let provider = core
        .provider(account.provider)
        .ok_or(RefreshFailure::NoProvider)?;
    let limit = std::time::Duration::try_from(FETCH_TIMEOUT).unwrap_or_default();
    match tokio::time::timeout(limit, provider.fetch_limits(account)).await {
        Ok(result) => Ok(result?),
        Err(_) => Err(RefreshFailure::Timeout),
    }
}

async fn persist(core: &Core, account: &AccountRef, snapshot: &LimitsSnapshot) {
    let id = account.id.clone();
    let stored = snapshot.clone();
    let result = core
        .storage
        .run(move |conn| {
            snapshots::save(conn, &id, &stored)?;
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
    let interval = model.settings.refresh_interval();
    policy::next_delay(Ok(()), 0, interval, core.random.unit())
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
    let failures = model.runtime_mut(&account.id).failures.saturating_add(1);
    let delay = policy::next_delay(Err(&failure), failures, interval, core.random.unit());
    let hold_until = policy::is_rate_limited(&failure)
        .then(|| now.checked_add(delay).ok())
        .flatten();
    model.record_failure(&account.id, failure, now, hold_until);
    delay
}

async fn review_alerts(core: &Core, account: &AccountRef, now: Timestamp) {
    let (record, snapshot, settings) = {
        let model = core.model();
        let record = model.account(&account.id).cloned();
        let snapshot = model.snapshots.get(&account.id).map(|e| e.snapshot.clone());
        (record, snapshot, model.settings.notifications)
    };
    let (Some(record), Some(snapshot)) = (record, snapshot) else {
        return;
    };
    let review = Review {
        account: &record,
        snapshot: &snapshot,
        settings,
        now,
    };
    if let Err(error) = core.alerts.review(&review).await {
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
