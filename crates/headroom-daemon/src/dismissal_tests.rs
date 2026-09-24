use std::path::PathBuf;
use std::sync::Arc;

use headroom_core::account::{AccountRef, CredentialOwner};
use headroom_core::provider::Provider;

use super::*;
use crate::model::Model;
use crate::registry::discover_all;
use crate::testing::{CODEX, FakeProvider, Harness, account, harness, session, snapshot};

fn cli(name: &str) -> AccountRef {
    AccountRef {
        home: PathBuf::from(format!("/home/ada/.codex-{name}")),
        ..account(CODEX, name)
    }
}

fn owned(name: &str) -> AccountRef {
    AccountRef {
        home: PathBuf::from(format!("/data/headroom/accounts/codex/{name}")),
        owner: CredentialOwner::Headroom,
        ..account(CODEX, name)
    }
}

async fn discovering(found: Vec<AccountRef>) -> (Harness, Arc<FakeProvider>) {
    let limits = snapshot(
        vec![session(20.0, "2026-09-23T12:00:00Z")],
        "2026-09-23T10:00:00Z",
    );
    let fake = Arc::new(FakeProvider::new(CODEX, found, limits));
    let provider: Arc<dyn Provider> = fake.clone();
    (harness(vec![provider]).await, fake)
}

async fn accounts() -> Harness {
    discovering(vec![cli("a"), cli("b"), owned("own")]).await.0
}

async fn dismissed(harness: &Harness, id: &str) {
    harness.core.dismiss_account(id).await.unwrap();
    discover_all(&harness.core).await;
}

async fn restored(harness: &Harness, provider: &str) {
    harness.core.restore_accounts(provider).await.unwrap();
    discover_all(&harness.core).await;
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

fn shown(harness: &Harness) -> Vec<(String, CredentialOwner)> {
    harness
        .core
        .state()
        .accounts
        .into_iter()
        .map(|a| (a.id, a.owner))
        .collect()
}

fn reloaded(harness: &Harness) -> Vec<String> {
    let model = harness.storage.blocking(|conn| Model::load(conn)).unwrap();
    model.active_accounts().map(|a| a.id().0.clone()).collect()
}

#[tokio::test]
async fn dismissed_accounts_leave_the_state_at_once_and_stay_dismissed_after_reload() {
    let harness = accounts().await;
    harness.core.dismiss_account("codex:a").await.unwrap();
    assert_eq!(listed(&harness), ["codex:b", "codex:own"]);
    assert!(
        harness
            .core
            .active_accounts()
            .iter()
            .all(|a| a.id.0 != "codex:a")
    );
    discover_all(&harness.core).await;
    assert_eq!(listed(&harness), ["codex:b", "codex:own"]);
    assert_eq!(reloaded(&harness), ["codex:b", "codex:own"]);
    let served = harness.core.settings_json().unwrap();
    assert!(!served.contains("dismissed"));
}

#[tokio::test]
async fn dismissing_twice_is_harmless() {
    let harness = accounts().await;
    dismissed(&harness, "codex:a").await;
    harness.core.dismiss_account("codex:a").await.unwrap();
    assert_eq!(harness.core.model().dismissed.iter().count(), 1);
}

#[tokio::test]
async fn dismissed_accounts_cannot_be_refreshed_or_relabelled() {
    let harness = accounts().await;
    harness.core.dismiss_account("codex:a").await.unwrap();
    assert!(matches!(
        harness.core.refresh("codex:a"),
        Err(CommandError::UnknownAccount(_))
    ));
    assert!(matches!(
        harness.core.set_account_label("codex:a", "work").await,
        Err(CommandError::UnknownAccount(_))
    ));
}

#[tokio::test]
async fn restoring_a_provider_brings_its_accounts_back() {
    let harness = accounts().await;
    dismissed(&harness, "codex:a").await;
    dismissed(&harness, "codex:b").await;
    assert_eq!(listed(&harness), ["codex:own"]);
    restored(&harness, "codex").await;
    assert_eq!(listed(&harness), ["codex:a", "codex:b", "codex:own"]);
    assert_eq!(reloaded(&harness), ["codex:a", "codex:b", "codex:own"]);
}

#[tokio::test]
async fn restoring_everything_clears_all_dismissals() {
    let harness = accounts().await;
    dismissed(&harness, "codex:a").await;
    restored(&harness, "").await;
    assert_eq!(harness.core.model().dismissed.iter().count(), 0);
    assert_eq!(listed(&harness), ["codex:a", "codex:b", "codex:own"]);
}

#[tokio::test]
async fn restoring_an_unknown_provider_is_rejected() {
    let harness = accounts().await;
    assert!(matches!(
        harness.core.restore_accounts("nope").await,
        Err(CommandError::UnknownProvider(_))
    ));
}

#[tokio::test]
async fn only_known_cli_owned_accounts_can_be_dismissed() {
    let harness = accounts().await;
    assert!(matches!(
        harness.core.dismiss_account("codex:own").await,
        Err(CommandError::NotDismissable(_))
    ));
    assert!(matches!(
        harness.core.dismiss_account("codex:nobody").await,
        Err(CommandError::UnknownAccount(_))
    ));
    assert_eq!(harness.core.model().dismissed.iter().count(), 0);
    assert_eq!(listed(&harness).len(), 3);
}

#[tokio::test]
async fn the_same_person_signed_in_through_headroom_shows_after_the_cli_home_is_dismissed() {
    let (harness, fake) = discovering(vec![cli("ada")]).await;
    dismissed(&harness, "codex:ada").await;
    assert!(listed(&harness).is_empty());
    let own = owned("ada");
    fake.accounts.lock().unwrap().push(own.clone());
    discover_all(&harness.core).await;
    assert_eq!(
        shown(&harness),
        [("codex:ada".to_owned(), CredentialOwner::Headroom)]
    );
    assert_eq!(harness.core.active_accounts(), [own]);
    assert!(matches!(
        harness.core.dismiss_account("codex:ada").await,
        Err(CommandError::NotDismissable(_))
    ));
}

#[tokio::test]
async fn restoring_prefers_the_cli_home_again() {
    let (harness, _) = discovering(vec![cli("ada"), owned("ada")]).await;
    assert_eq!(
        shown(&harness),
        [("codex:ada".to_owned(), CredentialOwner::Cli)]
    );
    dismissed(&harness, "codex:ada").await;
    assert_eq!(
        shown(&harness),
        [("codex:ada".to_owned(), CredentialOwner::Headroom)]
    );
    restored(&harness, "codex").await;
    assert_eq!(harness.core.active_accounts(), [cli("ada")]);
}

#[tokio::test]
async fn settings_writes_leave_dismissals_alone() {
    let harness = accounts().await;
    dismissed(&harness, "codex:a").await;
    let patch = r#"{"dismissed_accounts":[],"reduced_motion":true}"#;
    harness.core.update_settings(patch).await.unwrap();
    harness
        .core
        .set_settings(r#"{"dismissed_accounts":["codex:b"]}"#)
        .await
        .unwrap();
    discover_all(&harness.core).await;
    assert_eq!(listed(&harness), ["codex:b", "codex:own"]);
    assert_eq!(harness.core.model().dismissed.iter().count(), 1);
}
