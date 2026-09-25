use jiff::Timestamp;

use super::payload::{RefreshMode, RefreshReason, RefreshView};
use crate::model::{AccountRuntime, Model};
use crate::scheduler::policy;
use crate::storage::accounts::AccountRecord;

#[must_use]
pub fn refresh_view(
    record: &AccountRecord,
    runtime: Option<&AccountRuntime>,
    model: &Model,
    now: Timestamp,
) -> RefreshView {
    let active = model.activity.account_is_live(&record.reference, now);
    let live = policy::adaptive_live(&model.settings, active);
    RefreshView {
        mode: if live {
            RefreshMode::Live
        } else {
            RefreshMode::Idle
        },
        interval_secs: policy::effective_interval(&model.settings, live).as_secs(),
        next_at: runtime.and_then(|runtime| runtime.next_refresh_at),
        reason: reason(runtime, live),
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
