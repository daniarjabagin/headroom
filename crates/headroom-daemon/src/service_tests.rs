use std::sync::atomic::Ordering;

use headroom_core::provider::{Provider, ProviderError};

use super::*;
use crate::model::RefreshFailure;
use crate::rescan;
use crate::state::payload::AccountStatus;
use crate::testing::{CODEX, FakeProvider, Harness, account, eventually, harness};
use crate::testing::{session, snapshot};

struct Running {
    harness: Harness,
    provider: Arc<FakeProvider>,
    service: Service,
    task: tokio::task::JoinHandle<()>,
}

impl Drop for Running {
    fn drop(&mut self) {
        self.task.abort();
    }
}

async fn failing_with(error: ProviderError) -> Running {
    let limits = snapshot(
        vec![session(20.0, "2026-09-23T12:00:00Z")],
        "2026-09-23T10:00:00Z",
    );
    let provider = Arc::new(FakeProvider::new(CODEX, vec![account(CODEX, "a")], limits));
    *provider.limits.lock().unwrap() = Err(error.clone());
    let dynamic: Arc<dyn Provider> = provider.clone();
    let harness = harness(vec![dynamic]).await;
    let (rescans, requests) = rescan::channel();
    let task = tokio::spawn(crate::registry::supervise(harness.core.clone(), requests));
    let service = Service::new(harness.core.clone(), rescans);
    let expected = RefreshFailure::Provider(error);
    eventually(|| failure_of(&harness, "codex:a").as_ref() == Some(&expected)).await;
    Running {
        harness,
        provider,
        service,
        task,
    }
}

fn failure_of(harness: &Harness, id: &str) -> Option<RefreshFailure> {
    let id = AccountId(id.to_owned());
    let model = harness.core.model();
    model.runtime.get(&id).and_then(|r| r.failure.clone())
}

fn healthy(harness: &Harness, id: &str) -> bool {
    let id = AccountId(id.to_owned());
    let model = harness.core.model();
    model
        .runtime
        .get(&id)
        .is_some_and(|r| r.failure.is_none() && !r.refreshing && r.last_attempt.is_some())
}

fn status_of(harness: &Harness, id: &str) -> Option<AccountStatus> {
    let state = harness.core.state();
    state.accounts.iter().find(|a| a.id == id).map(|a| a.status)
}

fn listed(harness: &Harness) -> Vec<String> {
    let state = harness.core.state();
    state.accounts.into_iter().map(|a| a.id).collect()
}

fn succeed(provider: &FakeProvider) {
    *provider.limits.lock().unwrap() = Ok(snapshot(
        vec![session(20.0, "2026-09-23T12:00:00Z")],
        "2026-09-23T10:00:00Z",
    ));
}

#[tokio::test]
async fn retry_after_an_account_change_rescans_and_shows_the_new_account() {
    let run = failing_with(ProviderError::AccountChanged("moved".into())).await;
    *run.provider.accounts.lock().unwrap() = vec![account(CODEX, "b")];
    succeed(&run.provider);
    run.service.refresh("codex:a").unwrap();
    assert_eq!(
        status_of(&run.harness, "codex:a"),
        Some(AccountStatus::Refreshing)
    );
    eventually(|| listed(&run.harness) == ["codex:b"]).await;
    eventually(|| healthy(&run.harness, "codex:b")).await;
}

#[tokio::test]
async fn retry_of_a_signed_out_account_rescans_then_refreshes_it() {
    let run = failing_with(ProviderError::NotSignedIn).await;
    let before = run.provider.discoveries.load(Ordering::SeqCst);
    succeed(&run.provider);
    run.service.refresh("codex:a").unwrap();
    eventually(|| healthy(&run.harness, "codex:a")).await;
    assert_eq!(run.provider.discoveries.load(Ordering::SeqCst), before + 1);
}

#[tokio::test]
async fn retry_of_a_transient_error_refreshes_without_a_rescan() {
    let run = failing_with(ProviderError::Network("down".into())).await;
    let before = run.provider.discoveries.load(Ordering::SeqCst);
    succeed(&run.provider);
    run.service.refresh("codex:a").unwrap();
    eventually(|| healthy(&run.harness, "codex:a")).await;
    assert_eq!(run.provider.discoveries.load(Ordering::SeqCst), before);
}

#[tokio::test]
async fn unknown_accounts_are_rejected_before_any_rescan() {
    let run = failing_with(ProviderError::SignInExpired).await;
    assert!(matches!(
        run.service.refresh("codex:nobody"),
        Err(CommandError::UnknownAccount(_))
    ));
    run.service.refresh("").unwrap();
}
