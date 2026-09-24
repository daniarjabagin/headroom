use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use async_trait::async_trait;
use headroom_core::account::{AccountId, AccountRef};
use headroom_core::cursor::LogCursors;
use headroom_core::descriptor::ProviderDescriptor;
use headroom_core::event::UsageEvent;
use headroom_core::quota::LimitsSnapshot;
use tokio::sync::Semaphore;

use super::*;
use crate::rescan::Rescans;
use crate::state::payload::AccountStatus;
use crate::storage::snapshots;
use crate::testing::{CODEX, CODEX_DESCRIPTOR};
use crate::testing::{Harness, account, eventually, harness, session, snapshot};

struct GatedProvider {
    account: AccountRef,
    calls: AtomicUsize,
    gate: Semaphore,
}

#[async_trait]
impl Provider for GatedProvider {
    fn descriptor(&self) -> &'static ProviderDescriptor {
        &CODEX_DESCRIPTOR
    }

    async fn discover(&self) -> Result<Vec<AccountRef>, ProviderError> {
        Ok(vec![self.account.clone()])
    }

    async fn usage_homes(&self) -> Result<Vec<PathBuf>, ProviderError> {
        Ok(Vec::new())
    }

    async fn fetch_limits(&self, _account: &AccountRef) -> Result<LimitsSnapshot, ProviderError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.gate
            .acquire()
            .await
            .map_err(|_| ProviderError::Network("closed".into()))?
            .forget();
        Ok(snapshot(
            vec![session(95.0, "2026-09-23T12:00:00Z")],
            "2026-09-23T10:00:00Z",
        ))
    }

    fn read_usage(
        &self,
        _home: &Path,
        _cursors: &mut LogCursors,
    ) -> Result<Vec<UsageEvent>, ProviderError> {
        Ok(Vec::new())
    }
}

struct Running {
    harness: Harness,
    provider: Arc<GatedProvider>,
    rescans: Rescans,
    task: tokio::task::JoinHandle<()>,
}

impl Drop for Running {
    fn drop(&mut self) {
        self.task.abort();
    }
}

impl Running {
    fn calls(&self) -> usize {
        self.provider.calls.load(Ordering::SeqCst)
    }

    fn status(&self) -> Option<AccountStatus> {
        self.harness.core.state().accounts.first().map(|a| a.status)
    }

    fn refreshing(&self) -> bool {
        self.harness
            .core
            .model()
            .runtime
            .get(&id())
            .is_some_and(|runtime| runtime.refreshing)
    }

    async fn dismiss(&self) {
        self.harness.core.dismiss_account("codex:a").await.unwrap();
        self.rescans.rescan().await.unwrap();
    }
}

fn id() -> AccountId {
    account(CODEX, "a").id
}

async fn running(permits: usize) -> Running {
    let provider = Arc::new(GatedProvider {
        account: account(CODEX, "a"),
        calls: AtomicUsize::new(0),
        gate: Semaphore::new(permits),
    });
    let dynamic: Arc<dyn Provider> = provider.clone();
    let harness = harness(vec![dynamic]).await;
    let (rescans, requests) = rescan::channel();
    let task = tokio::spawn(supervise(harness.core.clone(), requests));
    eventually(|| provider.calls.load(Ordering::SeqCst) == 1).await;
    Running {
        harness,
        provider,
        rescans,
        task,
    }
}

#[tokio::test]
async fn dismissing_stops_the_worker_and_the_provider_is_not_called_again() {
    let running = running(100).await;
    eventually(|| running.status() == Some(AccountStatus::Fresh)).await;
    running.dismiss().await;
    assert!(!running.harness.core.has_trigger(&id()));
    running.harness.core.refresh_now();
    running.harness.core.trigger(&id());
    running.rescans.rescan().await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(running.calls(), 1);
    assert_eq!(running.status(), None);
}

#[tokio::test]
async fn restoring_starts_a_worker_that_refreshes() {
    let running = running(100).await;
    running.dismiss().await;
    running
        .harness
        .core
        .restore_accounts("codex")
        .await
        .unwrap();
    running.rescans.rescan().await.unwrap();
    assert!(running.harness.core.has_trigger(&id()));
    eventually(|| running.calls() == 2).await;
    eventually(|| running.status() == Some(AccountStatus::Fresh)).await;
}

#[tokio::test]
async fn a_refresh_finishing_after_dismissal_changes_nothing() {
    let running = running(0).await;
    assert!(running.refreshing());
    running
        .harness
        .core
        .dismiss_account("codex:a")
        .await
        .unwrap();
    running.provider.gate.add_permits(1);
    eventually(|| !running.refreshing()).await;
    let state = running.harness.core.state();
    assert!(state.accounts.is_empty());
    assert_eq!(state.headline, None);
    assert!(running.harness.notifier.texts().is_empty());
    assert!(!running.harness.core.model().snapshots.contains_key(&id()));
    let stored = running
        .harness
        .storage
        .blocking(|conn| snapshots::load_all(conn));
    assert!(stored.unwrap().is_empty());
}
