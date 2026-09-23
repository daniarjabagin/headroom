use std::time::Duration;

use headroom_core::account::ProviderKind;
use headroom_core::provider::Provider;

use super::*;
use crate::home::UsageHome;
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

fn first_home(core: &Core) -> UsageHome {
    core.model().usage_homes.first().unwrap().clone()
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
    let home = first_home(&harness.core);
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
    let home = first_home(&harness.core);
    let mut summarized_for = None;
    ingest::pass(&harness.core, &home, &mut summarized_for).await;
    let other = UsageHome {
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
    watchers.sync(&harness.core.model().usage_homes.clone());
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

#[tokio::test]
async fn refresh_now_ingests_every_usage_home_at_once() {
    let signed_in = tempfile::tempdir().unwrap();
    let api_key = tempfile::tempdir().unwrap();
    let codex = provider_at(signed_in.path());
    let claude = usage_only_provider(api_key.path());
    let providers: Vec<Arc<dyn Provider>> = vec![codex.clone(), claude.clone()];
    let harness = harness(providers).await;
    let mut watchers = UsageWatchers::new(harness.core.clone());
    watchers.sync(&harness.core.model().usage_homes.clone());
    eventually(|| harness.core.state().usage.len() == 2).await;
    tokio::time::sleep(Duration::from_millis(100)).await;
    codex
        .usage
        .lock()
        .unwrap()
        .push(event("a", "2026-09-23T09:00:00Z", "gpt-5.5", 100, 10));
    claude
        .usage
        .lock()
        .unwrap()
        .push(event("b", "2026-09-23T09:10:00Z", "claude-x", 40, 2));
    harness.core.refresh_now();
    eventually(|| harness.core.state().spend.today.total_tokens == 152).await;
}

fn usage_only_provider(home: &std::path::Path) -> Arc<FakeProvider> {
    let limits = snapshot(Vec::new(), "2026-09-23T10:00:00Z");
    let provider = FakeProvider::new(ProviderKind::Claude, Vec::new(), limits);
    *provider.homes.lock().unwrap() = vec![home.to_path_buf()];
    Arc::new(provider)
}

#[tokio::test]
async fn homes_without_accounts_are_ingested_and_counted_in_spend() {
    let signed_in = tempfile::tempdir().unwrap();
    let api_key = tempfile::tempdir().unwrap();
    let codex = provider_at(signed_in.path());
    let claude = usage_only_provider(api_key.path());
    let providers: Vec<Arc<dyn Provider>> = vec![codex.clone(), claude.clone()];
    let harness = harness(providers).await;
    codex
        .usage
        .lock()
        .unwrap()
        .push(event("a", "2026-09-23T09:00:00Z", "gpt-5.5", 100, 10));
    claude
        .usage
        .lock()
        .unwrap()
        .push(event("b", "2026-09-23T09:10:00Z", "claude-x", 40, 2));
    let homes = harness.core.model().usage_homes.clone();
    assert_eq!(homes.len(), 2);
    for home in &homes {
        ingest::pass(&harness.core, home, &mut None).await;
    }
    let state = harness.core.state();
    assert_eq!(state.accounts.len(), 1);
    let listed: Vec<_> = state
        .usage
        .iter()
        .map(|u| (u.provider, u.today.tokens.total))
        .collect();
    assert_eq!(
        listed,
        [(ProviderKind::Codex, 110), (ProviderKind::Claude, 42)]
    );
    assert_eq!(state.accounts[0].usage_home, state.usage[0].usage_home);
    assert_eq!(state.spend.today.total_tokens, 152);
    assert_eq!(state.spend.today.cost_usd_micros, 304);
}

#[tokio::test]
async fn watchers_follow_usage_homes_not_accounts() {
    let dir = tempfile::tempdir().unwrap();
    let provider = usage_only_provider(dir.path());
    provider
        .usage
        .lock()
        .unwrap()
        .push(event("a", "2026-09-23T09:00:00Z", "claude-x", 5, 5));
    let dynamic: Arc<dyn Provider> = provider.clone();
    let harness = harness(vec![dynamic]).await;
    let mut watchers = UsageWatchers::new(harness.core.clone());
    watchers.sync(&harness.core.model().usage_homes.clone());
    eventually(|| today_total(&harness.core) == 10).await;
    assert!(harness.core.state().accounts.is_empty());
    assert_eq!(watchers.tasks.len(), 1);
    provider.homes.lock().unwrap().clear();
    crate::registry::discover_all(&harness.core).await;
    watchers.sync(&harness.core.model().usage_homes.clone());
    assert!(watchers.tasks.is_empty());
    assert!(harness.core.state().usage.is_empty());
}
