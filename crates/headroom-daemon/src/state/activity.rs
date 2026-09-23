use headroom_core::quota::LimitsSource;
use jiff::Timestamp;

use crate::model::{Model, RefreshFailure};
use crate::storage::accounts::AccountRecord;

#[must_use]
pub fn next_refresh_at(model: &Model) -> Option<Timestamp> {
    visible(model)
        .filter_map(|record| model.runtime.get(record.id())?.next_refresh_at)
        .min()
}

#[must_use]
pub fn last_success_at(model: &Model) -> Option<Timestamp> {
    visible(model)
        .filter_map(|record| model.snapshots.get(record.id()))
        .filter(|entry| entry.snapshot.source == LimitsSource::Live)
        .map(|entry| entry.snapshot.fetched_at)
        .max()
}

#[must_use]
pub fn offline(model: &Model) -> bool {
    let mut accounts = model.active_accounts().peekable();
    accounts.peek().is_some() && accounts.all(|record| failed_on_network(model, record))
}

fn failed_on_network(model: &Model, record: &AccountRecord) -> bool {
    model
        .runtime
        .get(record.id())
        .and_then(|runtime| runtime.failure.as_ref())
        .is_some_and(RefreshFailure::is_network)
}

fn visible(model: &Model) -> impl Iterator<Item = &AccountRecord> {
    model.active_accounts().filter(|record| !record.hidden)
}

#[cfg(test)]
#[path = "activity_tests.rs"]
mod tests;
