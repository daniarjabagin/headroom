use std::fs;

use headroom_core::quota::LimitsSource;
use serde_json::json;
use wiremock::ResponseTemplate;

use super::test_support::{Setup, auth_document, server_with, write_auth, write_rollout};
use super::*;

const FULL: &str = include_str!("fixtures/usage_full.json");
const RECORDS: &str = include_str!("fixtures/rollout_records.jsonl");

#[tokio::test]
async fn discovers_cli_and_headroom_homes() {
    let setup = Setup::new();
    setup.sign_in("2026-09-24T00:00:00Z");
    let other = json!({ "tokens": { "access_token": "a", "account_id": "acct-other" } });
    write_auth(&setup.headroom_home("b"), &other);
    write_auth(&setup.headroom_home("c"), &auth_document("dup"));
    fs::create_dir_all(setup.headroom_home("a-empty")).unwrap();
    let accounts = setup.provider(DEFAULT_API_BASE).discover().await.unwrap();
    let summary: Vec<_> = accounts
        .iter()
        .map(|account| (account.home.clone(), account.owner))
        .collect();
    assert_eq!(
        summary,
        [
            (setup.cli_home(), CredentialOwner::Cli),
            (setup.headroom_home("b"), CredentialOwner::Headroom),
        ]
    );
    assert_eq!(accounts[0], setup.account());
}

#[tokio::test]
async fn discovery_reports_api_key_only_and_signed_out() {
    let setup = Setup::new();
    let provider = setup.provider(DEFAULT_API_BASE);
    assert_eq!(provider.discover().await, Err(ProviderError::NotSignedIn));
    write_auth(&setup.cli_home(), &json!({ "OPENAI_API_KEY": "sk-fake" }));
    assert_eq!(provider.discover().await, Err(ProviderError::ApiKeyOnly));
}

#[tokio::test]
async fn codex_home_override_is_used() {
    let setup = Setup::new();
    let custom = setup.root.path().join("custom");
    write_auth(&custom, &auth_document("opaque"));
    let mut provider = setup.provider(DEFAULT_API_BASE);
    provider.config.environment.codex_home = Some(custom.display().to_string());
    let accounts = provider.discover().await.unwrap();
    assert_eq!(accounts.len(), 1);
    assert_eq!(accounts[0].home, custom);
}

#[tokio::test]
async fn live_limits_come_from_the_usage_api() {
    let setup = Setup::new();
    setup.sign_in("2026-09-24T00:00:00Z");
    let server = server_with(
        ResponseTemplate::new(200).set_body_raw(FULL.as_bytes().to_vec(), "application/json"),
    )
    .await;
    let snapshot = setup
        .provider(&server.uri())
        .fetch_limits(&setup.account())
        .await
        .unwrap();
    assert_eq!(snapshot.source, LimitsSource::Live);
    assert_eq!(snapshot.windows.len(), 4);
    assert_eq!(
        snapshot.identity.email.as_deref(),
        Some("someone@example.com")
    );
    assert_eq!(snapshot.identity.plan.as_deref(), Some("Plus"));
}

#[tokio::test]
async fn credentials_file_is_never_modified() {
    let setup = Setup::new();
    setup.sign_in("2026-09-23T09:00:00Z");
    let before = fs::read(setup.cli_home().join("auth.json")).unwrap();
    let server = server_with(ResponseTemplate::new(401)).await;
    let _ = setup
        .provider(&server.uri())
        .fetch_limits(&setup.account())
        .await;
    assert_eq!(
        fs::read(setup.cli_home().join("auth.json")).unwrap(),
        before
    );
}

#[test]
fn read_usage_uses_the_injected_clock() {
    let setup = Setup::new();
    write_rollout(&setup.cli_home(), "sessions/rollout.jsonl", RECORDS);
    let mut cursors = LogCursors::default();
    let events = setup
        .provider(DEFAULT_API_BASE)
        .read_usage(&setup.cli_home(), &mut cursors)
        .unwrap();
    assert_eq!(events.len(), 3);
    assert_eq!(setup.provider(DEFAULT_API_BASE).kind(), ProviderKind::Codex);
}

#[tokio::test]
async fn api_key_home_with_sessions_is_a_usage_home() {
    let setup = Setup::new();
    write_auth(&setup.cli_home(), &json!({ "OPENAI_API_KEY": "sk-fake" }));
    write_rollout(&setup.cli_home(), "sessions/rollout.jsonl", RECORDS);
    let provider = setup.provider(DEFAULT_API_BASE);
    assert_eq!(provider.discover().await, Err(ProviderError::ApiKeyOnly));
    assert_eq!(provider.usage_homes().await.unwrap(), [setup.cli_home()]);
}

#[tokio::test]
async fn usage_homes_need_log_directories() {
    let setup = Setup::new();
    setup.sign_in("2026-09-24T00:00:00Z");
    fs::create_dir_all(setup.headroom_home("a").join("archived_sessions")).unwrap();
    write_auth(&setup.headroom_home("b"), &auth_document("other"));
    let homes = setup
        .provider(DEFAULT_API_BASE)
        .usage_homes()
        .await
        .unwrap();
    assert_eq!(homes, [setup.headroom_home("a")]);
}

#[tokio::test]
async fn usage_homes_are_deduplicated_by_canonical_path() {
    let setup = Setup::new();
    write_rollout(&setup.cli_home(), "sessions/rollout.jsonl", RECORDS);
    let data = setup.root.path().join("data/headroom/accounts/codex");
    fs::create_dir_all(&data).unwrap();
    std::os::unix::fs::symlink(setup.cli_home(), data.join("alias")).unwrap();
    let homes = setup
        .provider(DEFAULT_API_BASE)
        .usage_homes()
        .await
        .unwrap();
    assert_eq!(homes, [setup.cli_home()]);
}
