use std::fs;

use headroom_core::account::CredentialOwner;
use headroom_core::quota::WindowId;
use serde_json::json;
use tempfile::TempDir;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::*;

const FULL: &str = include_str!("fixtures/usage_full.json");
const NO_LIMITS: &str = include_str!("fixtures/usage_no_limits.json");
const FREE_CREDENTIALS: &str = include_str!("fixtures/credentials_free.json");

fn fixed_now() -> Timestamp {
    "2026-09-23T10:00:00Z".parse().unwrap()
}

fn write_json(path: &Path, value: &serde_json::Value) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, value.to_string()).unwrap();
}

fn sign_in(home: &Path, account: &str, expires_at: Timestamp) {
    write_json(
        &home.join(".claude.json"),
        &json!({ "oauthAccount": {
            "accountUuid": account,
            "organizationUuid": "org-1",
            "emailAddress": "someone@example.com"
        }}),
    );
    write_json(
        &home.join(".claude/.credentials.json"),
        &json!({ "claudeAiOauth": {
            "accessToken": "fake-token",
            "refreshToken": "fake-refresh",
            "expiresAt": expires_at.as_millisecond(),
            "subscriptionType": "max",
            "rateLimitTier": "default_claude_max_20x",
            "scopes": ["user:inference", "user:profile"]
        }}),
    );
}

async fn usage_server(expected_calls: u64) -> MockServer {
    usage_server_with(FULL, expected_calls).await
}

async fn usage_server_with(body: &str, expected_calls: u64) -> MockServer {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/oauth/usage"))
        .respond_with(ResponseTemplate::new(200).set_body_string(body))
        .expect(expected_calls)
        .mount(&server)
        .await;
    server
}

fn provider(home: &TempDir, server: &MockServer) -> ClaudeProvider {
    let mut config = ClaudeConfig::for_home(home.path().to_path_buf());
    config.api_base = server.uri();
    ClaudeProvider::with_clock(config, fixed_now).unwrap()
}

fn one_hour_later() -> Timestamp {
    fixed_now() + jiff::SignedDuration::from_hours(1)
}

#[tokio::test]
async fn discovered_account_fetches_live_limits() {
    let home = tempfile::tempdir().unwrap();
    sign_in(home.path(), "acc-1", one_hour_later());
    let server = usage_server(1).await;
    let provider = provider(&home, &server);
    let accounts = provider.discover().await.unwrap();
    assert_eq!(accounts.len(), 1);
    assert_eq!(accounts[0].owner, CredentialOwner::Cli);
    let snapshot = provider.fetch_limits(&accounts[0]).await.unwrap();
    assert_eq!(
        snapshot.identity.email.as_deref(),
        Some("someone@example.com")
    );
    assert_eq!(snapshot.identity.plan.as_deref(), Some("Max 20x"));
    assert_eq!(snapshot.identity.stable_key, "acc-1/org-1");
    assert_eq!(snapshot.identity.account_id(&ID), accounts[0].id);
    assert_eq!(snapshot.windows[0].id, WindowId::Session);
    assert_eq!(snapshot.windows.len(), 4);
    assert_eq!(snapshot.balances.len(), 2);
    assert_eq!(snapshot.fetched_at, fixed_now());
    assert_eq!(snapshot.source, LimitsSource::Live);
}

#[tokio::test]
async fn expired_token_fails_without_calling_the_api() {
    let home = tempfile::tempdir().unwrap();
    sign_in(home.path(), "acc-1", fixed_now());
    let server = usage_server(0).await;
    let provider = provider(&home, &server);
    let accounts = provider.discover().await.unwrap();
    assert_eq!(
        provider.fetch_limits(&accounts[0]).await.unwrap_err(),
        ProviderError::SignInExpired
    );
}

#[tokio::test]
async fn account_without_subscription_reports_no_subscription() {
    let home = tempfile::tempdir().unwrap();
    sign_in(home.path(), "acc-1", one_hour_later());
    fs::write(
        home.path().join(".claude/.credentials.json"),
        FREE_CREDENTIALS,
    )
    .unwrap();
    let server = usage_server_with(NO_LIMITS, 1).await;
    let provider = provider(&home, &server);
    let accounts = provider.discover().await.unwrap();
    assert_eq!(
        provider.fetch_limits(&accounts[0]).await.unwrap_err(),
        ProviderError::NoSubscription {
            detail: "No active Claude subscription.".into()
        }
    );
}

#[tokio::test]
async fn changed_identity_is_reported() {
    let home = tempfile::tempdir().unwrap();
    sign_in(home.path(), "acc-1", one_hour_later());
    let server = usage_server(0).await;
    let provider = provider(&home, &server);
    let accounts = provider.discover().await.unwrap();
    sign_in(home.path(), "acc-2", one_hour_later());
    assert!(matches!(
        provider.fetch_limits(&accounts[0]).await.unwrap_err(),
        ProviderError::LocalData(_)
    ));
}

#[tokio::test]
async fn missing_credentials_is_not_signed_in() {
    let home = tempfile::tempdir().unwrap();
    sign_in(home.path(), "acc-1", one_hour_later());
    fs::remove_file(home.path().join(".claude/.credentials.json")).unwrap();
    let server = usage_server(0).await;
    let provider = provider(&home, &server);
    let accounts = provider.discover().await.unwrap();
    assert_eq!(
        provider.fetch_limits(&accounts[0]).await.unwrap_err(),
        ProviderError::NotSignedIn
    );
}

#[test]
fn id_is_claude() {
    let config = ClaudeConfig::for_home("/nonexistent".into());
    let provider = ClaudeProvider::new(config).unwrap();
    assert_eq!(provider.id().as_str(), "claude");
    assert_eq!(provider.descriptor().display_name, "Claude");
}

#[tokio::test]
async fn a_keychain_signed_in_headroom_home_is_discovered_and_fetched() {
    let root = tempfile::tempdir().unwrap();
    let fake = crate::keychain::fake::FakeKeychain::new(root.path());
    let server = usage_server(1).await;
    let mut config = ClaudeConfig::for_home(root.path().join("home"));
    config.api_base = server.uri();
    config.user = Some("someone".to_owned());
    config.keychain = Some(fake.security());
    let home = config.headroom_accounts_dir().join("h1");
    write_json(
        &home.join(".claude.json"),
        &json!({ "oauthAccount": { "accountUuid": "acc-9", "organizationUuid": "org-1" } }),
    );
    let credentials = json!({ "claudeAiOauth": {
        "accessToken": "keychain-token",
        "expiresAt": one_hour_later().as_millisecond(),
        "subscriptionType": "max",
        "scopes": ["user:profile"]
    }});
    let service = keychain::scoped_service(home.to_str().unwrap());
    fake.insert(&service, Some("someone"), &credentials.to_string());
    let provider = ClaudeProvider::with_clock(config, fixed_now).unwrap();
    let account = provider.account_at(&home).await.unwrap().unwrap();
    assert_eq!(account.owner, CredentialOwner::Headroom);
    let discovered = provider.discover().await.unwrap();
    assert_eq!(discovered, std::slice::from_ref(&account));
    let snapshot = provider.fetch_limits(&account).await.unwrap();
    assert_eq!(snapshot.identity.stable_key, "acc-9/org-1");
    assert!(!fake.argv_log().contains("keychain-token"));
}
