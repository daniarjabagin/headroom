use std::sync::Arc;

use headroom_core::provider::Provider;

use super::*;
use crate::clock::Clock;
use crate::testing::{CODEX, FakeProvider, account, event, harness, session, snapshot, ts};
use crate::usage::ingest;

fn provider() -> Arc<FakeProvider> {
    let limits = snapshot(
        vec![session(10.0, "2026-09-23T12:00:00Z")],
        "2026-09-23T10:00:00Z",
    );
    Arc::new(FakeProvider::new(
        CODEX,
        vec![account(CODEX, "work")],
        limits,
    ))
}

async fn pass_with(at: &str) -> (bool, bool, Option<Timestamp>) {
    let provider = provider();
    provider
        .usage
        .lock()
        .unwrap()
        .push(event("a", at, "gpt-5.5", 100, 10));
    let dynamic: Arc<dyn Provider> = provider.clone();
    let harness = harness(vec![dynamic]).await;
    let home = harness.core.model().usage_homes.first().unwrap().clone();
    let changes = harness.core.activity_changes();
    ingest::pass(&harness.core, &home, &mut None).await;
    let model = harness.core.model();
    let live = model
        .activity
        .account_is_live(&account(CODEX, "work"), harness.clock.now());
    (
        live,
        changes.has_changed().unwrap(),
        model.activity.last_write(&home),
    )
}

#[tokio::test]
async fn fresh_log_records_make_the_home_live_and_wake_the_workers() {
    let (live, woke, last) = pass_with("2026-09-23T09:59:00Z").await;
    assert!(live);
    assert!(woke);
    assert_eq!(last, Some(ts("2026-09-23T09:59:00Z")));
}

#[tokio::test]
async fn catching_up_on_old_logs_does_not_make_the_home_live() {
    let (live, woke, last) = pass_with("2026-09-23T08:00:00Z").await;
    assert!(!live);
    assert!(!woke);
    assert_eq!(last, Some(ts("2026-09-23T08:00:00Z")));
}

#[tokio::test]
async fn records_dated_in_the_future_count_as_written_now() {
    let (live, _, last) = pass_with("2026-09-23T11:00:00Z").await;
    assert!(live);
    assert_eq!(last, Some(ts("2026-09-23T10:00:00Z")));
}
