use std::time::Duration;

use headroom_core::account::ProviderKind;
use headroom_core::provider::Provider;

use super::*;
use crate::storage::cursors;
use crate::testing::{FakeProvider, account, event, eventually, harness, session, snapshot};

fn provider_at(home: &std::path::Path) -> Arc<FakeProvider> {
    let mut work = account(ProviderKind::Codex, "work");
    work.home = home.to_path_buf();
    let limits = snapshot(
        vec![session(10.0, "2026-09-23T12:00:00Z")],
        "2026-09-23T10:00:00Z",
    );
    Arc::new(FakeProvider::new(ProviderKind::Codex, vec![work], limits))
}

fn today_total(core: &Core) -> u64 {
    core.state()
        .usage
        .first()
        .map_or(0, |usage| usage.today.tokens.total)
}

#[tokio::test]
async fn a_pass_ingests_events_cursors_and_summary() {
    let dir = tempfile::tempdir().unwrap();
    let provider = provider_at(dir.path());
    provider.usage.lock().unwrap().extend([
        event("a", "2026-09-23T09:00:00Z", "gpt-5.5", 100, 10),
        event("a", "2026-09-23T09:00:00Z", "gpt-5.5", 100, 5),
        event("b", "2026-09-23T09:30:00Z", "gpt-5.5", 50, 5),
    ]);
    let dynamic: Arc<dyn Provider> = provider.clone();
    let harness = harness(vec![dynamic]).await;
    let home = harness
        .core
        .model()
        .usage_homes()
        .into_iter()
        .next()
        .unwrap();
    let mut summarized_for = None;
    ingest::pass(&harness.core, &home, &mut summarized_for).await;
    assert_eq!(today_total(&harness.core), 165);
    assert_eq!(
        summarized_for.map(|d| d.to_string()).as_deref(),
        Some("2026-09-23")
    );
    let stored = harness
        .storage
        .blocking(|conn| cursors::load(conn, &home))
        .unwrap();
    assert_eq!(stored.0.values().next().unwrap().offset, 3);
    assert_eq!(
        harness.core.state().usage[0].usage_home,
        dir.path().display().to_string()
    );
}

#[tokio::test]
async fn homes_without_a_provider_are_rejected_without_losing_usage() {
    let dir = tempfile::tempdir().unwrap();
    let provider = provider_at(dir.path());
    provider
        .usage
        .lock()
        .unwrap()
        .push(event("a", "2026-09-23T09:00:00Z", "gpt-5.5", 100, 10));
    let dynamic: Arc<dyn Provider> = provider.clone();
    let harness = harness(vec![dynamic]).await;
    let home = harness
        .core
        .model()
        .usage_homes()
        .into_iter()
        .next()
        .unwrap();
    let mut summarized_for = None;
    ingest::pass(&harness.core, &home, &mut summarized_for).await;
    let other = crate::home::UsageHome {
        provider: ProviderKind::Claude,
        home: dir.path().to_path_buf(),
    };
    assert!(matches!(
        ingest::ingest(&harness.core, &other).await,
        Err(ingest::IngestError::NoProvider(_))
    ));
    assert_eq!(today_total(&harness.core), 110);
}

#[tokio::test]
async fn file_changes_trigger_a_debounced_ingest() {
    let dir = tempfile::tempdir().unwrap();
    let provider = provider_at(dir.path());
    let dynamic: Arc<dyn Provider> = provider.clone();
    let harness = harness(vec![dynamic]).await;
    let mut watchers = UsageWatchers::new(harness.core.clone());
    watchers.sync(&harness.core.model().usage_homes());
    eventually(|| harness.core.state().usage.len() == 1).await;
    provider
        .usage
        .lock()
        .unwrap()
        .push(event("late", "2026-09-23T09:59:00Z", "gpt-5.5", 7, 3));
    tokio::time::sleep(Duration::from_millis(200)).await;
    std::fs::write(dir.path().join("session.jsonl"), "{}\n").unwrap();
    for _ in 0..60 {
        if today_total(&harness.core) == 10 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    assert_eq!(today_total(&harness.core), 10);
    watchers.sync(&std::collections::BTreeSet::new());
    assert!(watchers.tasks.is_empty());
}
