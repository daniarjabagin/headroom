use std::path::Path;
use std::sync::Arc;

use headroom_core::provider::{Provider, ProviderError};

use super::*;
use crate::model::RefreshFailure;
use crate::rescan;
use crate::testing::{CODEX, FakeProvider, Harness, account, eventually, harness};
use crate::testing::{session, snapshot};

const FAST: WatchTiming = WatchTiming {
    sync_every: Duration::from_millis(20),
    settle: Duration::from_millis(20),
};

struct Running {
    harness: Harness,
    provider: Arc<FakeProvider>,
    service: Service,
    supervisor: JoinHandle<()>,
}

impl Drop for Running {
    fn drop(&mut self) {
        self.supervisor.abort();
    }
}

fn id() -> AccountId {
    AccountId("codex:work".into())
}

async fn signed_out_at(home: &Path) -> Running {
    let mut work = account(CODEX, "work");
    work.home = home.to_path_buf();
    let limits = snapshot(
        vec![session(20.0, "2026-09-23T12:00:00Z")],
        "2026-09-23T10:00:00Z",
    );
    let provider = Arc::new(FakeProvider::new(CODEX, vec![work], limits));
    *provider.limits.lock().unwrap() = Err(ProviderError::SignInExpired);
    let dynamic: Arc<dyn Provider> = provider.clone();
    let harness = harness(vec![dynamic]).await;
    let (rescans, requests) = rescan::channel();
    let supervisor = tokio::spawn(crate::registry::supervise(harness.core.clone(), requests));
    let service = Service::new(harness.core.clone(), rescans);
    let expired = RefreshFailure::Provider(ProviderError::SignInExpired);
    eventually(|| failure(&harness).as_ref() == Some(&expired)).await;
    Running {
        harness,
        provider,
        service,
        supervisor,
    }
}

fn failure(harness: &Harness) -> Option<RefreshFailure> {
    let model = harness.core.model();
    model.runtime.get(&id()).and_then(|r| r.failure.clone())
}

fn healthy(harness: &Harness) -> bool {
    let model = harness.core.model();
    model
        .runtime
        .get(&id())
        .is_some_and(|r| r.failure.is_none() && !r.refreshing)
}

#[tokio::test]
async fn a_credential_change_revives_a_signed_out_account() {
    let home = tempfile::tempdir().unwrap();
    let run = signed_out_at(home.path()).await;
    let watcher = tokio::spawn(watch_signed_out(run.service.clone(), FAST));
    *run.provider.limits.lock().unwrap() = Ok(snapshot(
        vec![session(30.0, "2026-09-23T12:00:00Z")],
        "2026-09-23T10:00:00Z",
    ));
    for attempt in 0..200 {
        if healthy(&run.harness) {
            break;
        }
        std::fs::write(
            home.path().join("auth.json"),
            format!("{{\"n\":{attempt}}}"),
        )
        .unwrap();
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    watcher.abort();
    assert!(healthy(&run.harness));
}

#[tokio::test]
async fn watches_follow_the_failing_accounts_only() {
    let home = tempfile::tempdir().unwrap();
    let run = signed_out_at(home.path()).await;
    let mut watches = HashMap::new();
    sync(&run.service, &mut watches, FAST.settle);
    let paths: Vec<&PathBuf> = watches.values().map(|w| &w.path).collect();
    assert_eq!(paths, [&home.path().join("auth.json")]);
    run.harness.core.model().runtime_mut(&id()).failure = None;
    sync(&run.service, &mut watches, FAST.settle);
    assert!(watches.is_empty());
}

#[tokio::test]
async fn nothing_is_refreshed_while_the_credentials_stay_untouched() {
    let home = tempfile::tempdir().unwrap();
    let run = signed_out_at(home.path()).await;
    let watcher = tokio::spawn(watch_signed_out(run.service.clone(), FAST));
    std::fs::write(home.path().join("history.jsonl"), "{}\n").unwrap();
    tokio::time::sleep(Duration::from_millis(200)).await;
    watcher.abort();
    let expired = RefreshFailure::Provider(ProviderError::SignInExpired);
    assert_eq!(failure(&run.harness), Some(expired));
    let model = run.harness.core.model();
    assert_eq!(model.runtime[&id()].failures, 1);
}
