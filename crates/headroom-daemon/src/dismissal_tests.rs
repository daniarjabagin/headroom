use std::sync::Arc;

use headroom_core::account::AccountRef;
use headroom_core::provider::Provider;

use super::*;
use crate::model::Model;
use crate::registry::discover_all;
use crate::testing::{CODEX, FakeProvider, Harness, account, harness, session, snapshot};

fn owned_by_headroom() -> AccountRef {
    AccountRef {
        owner: CredentialOwner::Headroom,
        ..account(CODEX, "own")
    }
}

async fn accounts() -> Harness {
    let limits = snapshot(
        vec![session(20.0, "2026-09-23T12:00:00Z")],
        "2026-09-23T10:00:00Z",
    );
    let found = vec![
        account(CODEX, "a"),
        account(CODEX, "b"),
        owned_by_headroom(),
    ];
    let provider: Arc<dyn Provider> = Arc::new(FakeProvider::new(CODEX, found, limits));
    harness(vec![provider]).await
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

fn reloaded(harness: &Harness) -> Vec<String> {
    let model = harness.storage.blocking(|conn| Model::load(conn)).unwrap();
    model.active_accounts().map(|a| a.id().0.clone()).collect()
}

#[tokio::test]
async fn dismissed_accounts_leave_the_state_and_stay_dismissed_after_reload() {
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
    assert_eq!(reloaded(&harness), ["codex:b", "codex:own"]);
    discover_all(&harness.core).await;
    assert_eq!(listed(&harness), ["codex:b", "codex:own"]);
    let served = harness.core.settings_json().unwrap();
    assert!(served.contains(r#""dismissed_accounts":["codex:a"]"#));
}

#[tokio::test]
async fn dismissing_twice_is_harmless() {
    let harness = accounts().await;
    harness.core.dismiss_account("codex:a").await.unwrap();
    harness.core.dismiss_account("codex:a").await.unwrap();
    assert_eq!(harness.core.model().settings.dismissed_accounts.len(), 1);
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
    harness.core.dismiss_account("codex:a").await.unwrap();
    harness.core.dismiss_account("codex:b").await.unwrap();
    assert_eq!(listed(&harness), ["codex:own"]);
    harness.core.restore_accounts("codex").await.unwrap();
    assert_eq!(listed(&harness), ["codex:a", "codex:b", "codex:own"]);
    assert_eq!(reloaded(&harness), ["codex:a", "codex:b", "codex:own"]);
}

#[tokio::test]
async fn restoring_everything_clears_all_dismissals() {
    let harness = accounts().await;
    harness.core.dismiss_account("codex:a").await.unwrap();
    harness.core.restore_accounts("").await.unwrap();
    assert!(harness.core.model().settings.dismissed_accounts.is_empty());
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
    assert!(harness.core.model().settings.dismissed_accounts.is_empty());
    assert_eq!(listed(&harness).len(), 3);
}
