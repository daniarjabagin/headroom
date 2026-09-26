use std::path::PathBuf;
use std::sync::Arc;

use headroom_core::account::AccountId;
use headroom_core::provider::Provider;

use super::*;
use crate::clock::Clock;
use crate::state::payload::RefreshMode;
use crate::testing::{CLAUDE, CODEX, FakeProvider, Harness, event, harness, session, snapshot};
use crate::usage::ingest;

const NOW: &str = "2026-09-23T10:00:00Z";

fn cli(provider: &ProviderId, name: &str, home: &str) -> AccountRef {
    AccountRef {
        id: AccountId(format!("{provider}:{name}")),
        provider: provider.clone(),
        home: PathBuf::from(home),
        owner: CredentialOwner::Cli,
    }
}

fn owned(provider: &ProviderId, name: &str) -> AccountRef {
    AccountRef {
        home: PathBuf::from(format!(
            "/home/ada/.local/share/headroom/accounts/{provider}/{name}"
        )),
        owner: CredentialOwner::Headroom,
        ..cli(provider, name, "/unused")
    }
}

fn cli_home(provider: &ProviderId) -> String {
    format!("/home/ada/.{provider}")
}

#[test]
fn a_hidden_cli_home_of_the_same_identity_links_to_the_headroom_account() {
    let own = owned(&CLAUDE, "ada");
    let sign_ins = [cli(&CLAUDE, "ada", "/home/ada/.claude")];
    let linked = linked_homes(&sign_ins, &[&own], &own);
    assert_eq!(linked, [Path::new("/home/ada/.claude")]);
}

#[test]
fn another_identity_is_never_linked() {
    let own = owned(&CLAUDE, "ada");
    let sign_ins = [cli(&CLAUDE, "bob", "/home/ada/.claude")];
    assert!(linked_homes(&sign_ins, &[&own], &own).is_empty());
}

#[test]
fn a_cli_home_shown_as_its_own_account_is_not_linked() {
    let own = owned(&CODEX, "ada");
    let other = cli(&CODEX, "bob", "/home/ada/.codex");
    let sign_ins = [cli(&CODEX, "ada", "/home/ada/.codex"), other.clone()];
    assert!(linked_homes(&sign_ins, &[&own, &other], &own).is_empty());
}

#[test]
fn only_headroom_accounts_take_cli_homes() {
    let shown = cli(&CLAUDE, "ada", "/home/ada/.claude-work");
    let sign_ins = [cli(&CLAUDE, "ada", "/home/ada/.claude")];
    assert!(linked_homes(&sign_ins, &[&shown], &shown).is_empty());
}

#[test]
fn sign_ins_keep_only_cli_homes_of_the_provider() {
    let mut sign_ins = CliSignIns::default();
    let found = [
        cli(&CLAUDE, "ada", "/home/ada/.claude"),
        owned(&CLAUDE, "ada"),
        cli(&CODEX, "ada", "/home/ada/.codex"),
    ];
    sign_ins.set(&CLAUDE, &found);
    assert_eq!(sign_ins.of(&CLAUDE), &found[..1]);
    assert!(sign_ins.of(&CODEX).is_empty());
}

async fn signed_in_twice(provider: &ProviderId, cli_name: &str) -> (Harness, Arc<FakeProvider>) {
    let accounts = vec![
        cli(provider, cli_name, &cli_home(provider)),
        owned(provider, "ada"),
    ];
    let limits = snapshot(vec![session(10.0, "2026-09-23T12:00:00Z")], NOW);
    let fake = Arc::new(FakeProvider::new(provider.clone(), accounts, limits));
    *fake.homes.lock().unwrap() = vec![PathBuf::from(cli_home(provider))];
    let dynamic: Arc<dyn Provider> = fake.clone();
    (harness(vec![dynamic]).await, fake)
}

async fn dismiss_cli(harness: &Harness, provider: &ProviderId, name: &str) {
    let id = format!("{provider}:{name}");
    harness.core.dismiss_account(&id).await.unwrap();
    crate::registry::discover_all(&harness.core).await;
}

async fn write_log(harness: &Harness, fake: &FakeProvider, provider: &ProviderId) {
    fake.usage
        .lock()
        .unwrap()
        .push(event("a", "2026-09-23T09:59:00Z", "model", 100, 10));
    let home = UsageHome {
        provider: provider.clone(),
        home: PathBuf::from(cli_home(provider)),
    };
    ingest::pass(&harness.core, &home, &mut None).await;
}

async fn attributed_after_dismissal(provider: ProviderId) {
    let (harness, fake) = signed_in_twice(&provider, "ada").await;
    dismiss_cli(&harness, &provider, "ada").await;
    write_log(&harness, &fake, &provider).await;
    let state = harness.core.state();
    assert_eq!(state.accounts.len(), 1);
    let account = &state.accounts[0];
    assert_eq!(account.owner, CredentialOwner::Headroom);
    assert_eq!(account.usage_home, format!("~/.{provider}"));
    assert_eq!(state.usage.len(), 1);
    assert_eq!(state.usage[0].usage_home, account.usage_home);
    assert_eq!(state.usage[0].today.tokens.total, 110);
    assert_eq!(account.refresh.as_ref().unwrap().mode, RefreshMode::Live);
}

#[tokio::test]
async fn dismissed_claude_cli_home_feeds_the_headroom_account() {
    attributed_after_dismissal(CLAUDE).await;
}

#[tokio::test]
async fn dismissed_codex_cli_home_feeds_the_headroom_account() {
    attributed_after_dismissal(CODEX).await;
}

#[tokio::test]
async fn a_visible_cli_account_keeps_its_home_and_is_counted_once() {
    let (harness, fake) = signed_in_twice(&CLAUDE, "ada").await;
    write_log(&harness, &fake, &CLAUDE).await;
    let state = harness.core.state();
    assert_eq!(state.accounts.len(), 1);
    assert_eq!(state.accounts[0].owner, CredentialOwner::Cli);
    assert_eq!(state.accounts[0].usage_home, "~/.claude");
    assert_eq!(state.usage.len(), 1);
    assert_eq!(state.usage[0].today.tokens.total, 110);
}

#[tokio::test]
async fn a_cli_home_of_another_identity_is_not_attributed() {
    let (harness, fake) = signed_in_twice(&CLAUDE, "bob").await;
    dismiss_cli(&harness, &CLAUDE, "bob").await;
    write_log(&harness, &fake, &CLAUDE).await;
    let state = harness.core.state();
    assert_eq!(state.accounts.len(), 1);
    let account = &state.accounts[0];
    assert_eq!(
        account.usage_home,
        "~/.local/share/headroom/accounts/claude/ada"
    );
    assert_eq!(account.refresh.as_ref().unwrap().mode, RefreshMode::Idle);
    let own = owned(&CLAUDE, "ada");
    assert!(
        !harness
            .core
            .model()
            .account_is_live(&own, harness.clock.now())
    );
}

#[tokio::test]
async fn restoring_the_cli_account_ends_the_link_at_once() {
    let (harness, _fake) = signed_in_twice(&CLAUDE, "ada").await;
    dismiss_cli(&harness, &CLAUDE, "ada").await;
    harness.core.restore_accounts("claude").await.unwrap();
    crate::registry::discover_all(&harness.core).await;
    let state = harness.core.state();
    assert_eq!(state.accounts.len(), 1);
    assert_eq!(state.accounts[0].owner, CredentialOwner::Cli);
    assert_eq!(state.accounts[0].usage_home, "~/.claude");
}

#[path = "log_homes_stored_tests.rs"]
mod stored;
