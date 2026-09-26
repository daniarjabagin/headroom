use jiff::Timestamp;

use super::payload::{RefreshMode, RefreshReason, RefreshView};
use crate::catalog::ProviderCatalog;
use crate::model::{AccountRuntime, Model};
use crate::scheduler::policy;
use crate::storage::accounts::AccountRecord;

#[must_use]
pub fn refresh_view(
    record: &AccountRecord,
    runtime: Option<&AccountRuntime>,
    model: &Model,
    catalog: &ProviderCatalog,
    now: Timestamp,
) -> RefreshView {
    let active = model.account_is_live(&record.reference, now);
    let live = policy::adaptive_live(&model.settings, active);
    let min_poll = catalog.min_poll_interval(&record.reference.provider);
    RefreshView {
        mode: if live {
            RefreshMode::Live
        } else {
            RefreshMode::Idle
        },
        interval_secs: policy::provider_interval(&model.settings, live, min_poll).as_secs(),
        next_at: runtime.and_then(|runtime| runtime.next_refresh_at),
        reason: reason(runtime, live),
        last_attempt_at: runtime.and_then(|runtime| runtime.last_attempt),
    }
}

fn reason(runtime: Option<&AccountRuntime>, live: bool) -> RefreshReason {
    match runtime.and_then(|runtime| runtime.failure.as_ref()) {
        Some(failure) if policy::holds_soft_refresh(failure) => RefreshReason::Hold,
        Some(_) => RefreshReason::Backoff,
        None if live => RefreshReason::Activity,
        None => RefreshReason::Schedule,
    }
}

#[cfg(test)]
#[path = "refresh_tests.rs"]
mod tests;
