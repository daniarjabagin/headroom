use std::sync::atomic::Ordering;

use super::*;
use crate::rescan::Rescans;
use crate::testing::CODEX;
use crate::testing::{FakeProvider, Harness, account, eventually, harness, session, snapshot};

struct Running {
    harness: Harness,
    provider: Arc<FakeProvider>,
    rescans: Rescans,
    task: tokio::task::JoinHandle<()>,
}

impl Drop for Running {
    fn drop(&mut self) {
        self.task.abort();
    }
}

async fn running() -> Running {
    let limits = snapshot(
        vec![session(20.0, "2026-09-23T12:00:00Z")],
        "2026-09-23T10:00:00Z",
    );
    let accounts = vec![account(CODEX, "a")];
    let provider = Arc::new(FakeProvider::new(CODEX, accounts, limits));
    let dynamic: Arc<dyn Provider> = provider.clone();
    let harness = harness(vec![dynamic]).await;
    let (rescans, requests) = rescan::channel();
    let task = tokio::spawn(supervise(harness.core.clone(), requests));
    eventually(|| provider.discoveries.load(Ordering::SeqCst) == 2).await;
    eventually(|| harness.core.state().usage.len() == 1).await;
    Running {
        harness,
        provider,
        rescans,
        task,
    }
}

fn listed(harness: &Harness) -> Vec<String> {
    harness
        .core
        .state()
        .accounts
        .into_iter()
        .map(|a| a.id)
        .collect()
}

fn refreshed(harness: &Harness, id: &str) -> bool {
    let id = headroom_core::account::AccountId(id.to_owned());
    harness
        .core
        .model()
        .runtime
        .get(&id)
        .is_some_and(|r| r.last_attempt.is_some() && !r.refreshing)
}

#[tokio::test]
async fn rescan_lists_new_accounts_before_returning_and_refreshes_them() {
    let run = running().await;
    run.provider
        .accounts
        .lock()
        .unwrap()
        .push(account(CODEX, "b"));
    run.rescans.rescan().await.unwrap();
    assert_eq!(listed(&run.harness), ["codex:a", "codex:b"]);
    eventually(|| refreshed(&run.harness, "codex:b")).await;
}

#[tokio::test]
async fn rescan_drops_removed_accounts() {
    let run = running().await;
    run.provider.accounts.lock().unwrap().clear();
    run.rescans.rescan().await.unwrap();
    assert!(listed(&run.harness).is_empty());
}

#[tokio::test]
async fn concurrent_rescans_share_one_discovery() {
    let run = running().await;
    let before = run.provider.discoveries.load(Ordering::SeqCst);
    let (a, b, c) = tokio::join!(
        run.rescans.rescan(),
        run.rescans.rescan(),
        run.rescans.rescan()
    );
    assert!(a.is_ok() && b.is_ok() && c.is_ok());
    assert_eq!(run.provider.discoveries.load(Ordering::SeqCst), before + 1);
}

#[tokio::test]
async fn rescan_fails_once_the_supervisor_is_gone() {
    let (rescans, requests) = rescan::channel();
    drop(requests);
    assert!(matches!(
        rescans.rescan().await,
        Err(crate::error::CommandError::Stopping)
    ));
}

#[tokio::test]
async fn rescan_starts_reading_new_usage_homes() {
    let run = running().await;
    let dir = tempfile::tempdir().unwrap();
    run.provider.homes.lock().unwrap().clear();
    run.provider.homes.lock().unwrap().push(dir.path().into());
    run.provider
        .usage
        .lock()
        .unwrap()
        .push(crate::testing::event(
            "a",
            "2026-09-23T09:00:00Z",
            "gpt-5.5",
            30,
            3,
        ));
    run.rescans.rescan().await.unwrap();
    let shown = dir.path().display().to_string();
    eventually(|| {
        let state = run.harness.core.state();
        state.usage.len() == 1 && state.usage[0].usage_home == shown
    })
    .await;
    assert_eq!(run.harness.core.state().spend.today.total_tokens, 33);
}
