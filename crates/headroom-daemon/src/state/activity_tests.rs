use headroom_core::account::{AccountId, ProviderKind};
use headroom_core::provider::ProviderError;
use headroom_core::quota::LimitsSource;

use super::*;
use crate::model::{AccountRuntime, SnapshotEntry, SnapshotOrigin};
use crate::testing::{account, snapshot, ts};

const NOW: &str = "2026-09-23T10:00:00Z";

fn record(name: &str, hidden: bool, gone: bool) -> AccountRecord {
    AccountRecord {
        reference: account(ProviderKind::Codex, name),
        label: None,
        hidden,
        sort_order: 0,
        email: None,
        plan: None,
        last_seen: ts(NOW),
        gone,
    }
}

fn model(records: Vec<AccountRecord>) -> Model {
    Model {
        accounts: records,
        ..Model::default()
    }
}

fn id(name: &str) -> AccountId {
    account(ProviderKind::Codex, name).id
}

fn scheduled(model: &mut Model, name: &str, at: &str) {
    model.runtime_mut(&id(name)).next_refresh_at = Some(ts(at));
}

fn failed(model: &mut Model, name: &str, failure: RefreshFailure) {
    model.record_failure(&id(name), failure, ts(NOW), None);
}

fn network() -> RefreshFailure {
    RefreshFailure::Provider(ProviderError::Network("down".into()))
}

fn fetched(model: &mut Model, name: &str, at: &str, origin: SnapshotOrigin) {
    let entry = SnapshotEntry {
        snapshot: snapshot(Vec::new(), at),
        origin,
    };
    model.snapshots.insert(id(name), entry);
}

#[test]
fn next_refresh_is_the_earliest_among_visible_accounts() {
    let mut m = model(vec![
        record("a", false, false),
        record("b", false, false),
        record("hidden", true, false),
        record("gone", false, true),
        record("idle", false, false),
    ]);
    scheduled(&mut m, "a", "2026-09-23T10:05:00Z");
    scheduled(&mut m, "b", "2026-09-23T10:03:00Z");
    scheduled(&mut m, "hidden", "2026-09-23T10:01:00Z");
    scheduled(&mut m, "gone", "2026-09-23T10:00:30Z");
    assert_eq!(next_refresh_at(&m), Some(ts("2026-09-23T10:03:00Z")));
}

#[test]
fn next_refresh_is_null_without_a_schedule() {
    let m = model(vec![record("a", false, false)]);
    assert_eq!(next_refresh_at(&m), None);
    assert_eq!(next_refresh_at(&Model::default()), None);
}

#[test]
fn last_success_is_the_newest_live_snapshot_of_visible_accounts() {
    let mut m = model(vec![
        record("a", false, false),
        record("b", false, false),
        record("local", false, false),
        record("hidden", true, false),
    ]);
    fetched(&mut m, "a", "2026-09-23T09:40:00Z", SnapshotOrigin::Cache);
    fetched(
        &mut m,
        "b",
        "2026-09-23T09:50:00Z",
        SnapshotOrigin::Refreshed,
    );
    fetched(
        &mut m,
        "hidden",
        "2026-09-23T09:59:00Z",
        SnapshotOrigin::Refreshed,
    );
    fetched(
        &mut m,
        "local",
        "2026-09-23T09:58:00Z",
        SnapshotOrigin::Refreshed,
    );
    let local = m.snapshots.get_mut(&id("local")).unwrap();
    local.snapshot.source = LimitsSource::LocalLog {
        observed_at: ts("2026-09-23T09:58:00Z"),
    };
    assert_eq!(last_success_at(&m), Some(ts("2026-09-23T09:50:00Z")));
    assert_eq!(last_success_at(&Model::default()), None);
}

#[test]
fn offline_when_every_active_account_failed_on_the_network() {
    let mut m = model(vec![
        record("a", false, false),
        record("hidden", true, false),
        record("gone", false, true),
    ]);
    failed(&mut m, "a", network());
    assert!(!offline(&m));
    failed(&mut m, "hidden", network());
    assert!(offline(&m));
}

#[test]
fn a_newer_success_or_another_error_is_not_offline() {
    let mut m = model(vec![record("a", false, false), record("b", false, false)]);
    failed(&mut m, "a", network());
    failed(&mut m, "b", RefreshFailure::Timeout);
    assert!(!offline(&m));
    failed(&mut m, "b", network());
    assert!(offline(&m));
    let later = ts("2026-09-23T10:01:00Z");
    m.record_success(
        &id("b"),
        snapshot(Vec::new(), "2026-09-23T10:01:00Z"),
        later,
    );
    assert!(!offline(&m));
}

#[test]
fn no_accounts_or_untried_accounts_are_not_offline() {
    assert!(!offline(&Model::default()));
    let mut m = model(vec![record("a", false, false), record("b", false, false)]);
    failed(&mut m, "a", network());
    m.runtime.insert(id("b"), AccountRuntime::default());
    assert!(!offline(&m));
}
