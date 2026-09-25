use std::fs;

use serde_json::{Value, json};
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::*;

const FULL: &str = include_str!("fixtures/usage_full.json");

fn fixed_now() -> Timestamp {
    "2026-09-23T10:00:00Z".parse().unwrap()
}

fn write_json(path: &Path, value: &Value) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, value.to_string()).unwrap();
}

fn sign_in(dir: &Path, identity_file: &Path) {
    write_json(
        identity_file,
        &json!({ "oauthAccount": { "accountUuid": "acc-1", "organizationUuid": "org-1" } }),
    );
    write_json(
        &dir.join(".credentials.json"),
        &json!({ "claudeAiOauth": {
            "accessToken": "expired-token",
            "refreshToken": "fake-refresh",
            "expiresAt": fixed_now().as_millisecond() - 1,
            "subscriptionType": "max",
            "scopes": ["user:inference", "user:profile"]
        }}),
    );
}

async fn server(token_calls: u64) -> MockServer {
    let server = MockServer::start().await;
    let granted = json!({
        "access_token": "fresh-token",
        "refresh_token": "fresh-refresh",
        "expires_in": 28_800,
        "scope": "user:inference user:profile"
    });
    Mock::given(method("POST"))
        .and(path("/v1/oauth/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(granted))
        .expect(token_calls)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/oauth/usage"))
        .and(header("authorization", "Bearer fresh-token"))
        .respond_with(ResponseTemplate::new(200).set_body_string(FULL))
        .mount(&server)
        .await;
    server
}

fn provider(root: &Path, server: &MockServer) -> ClaudeProvider {
    let mut config = ClaudeConfig::for_home(root.join("home"));
    config.api_base = server.uri();
    config.token_url = format!("{}/v1/oauth/token", server.uri());
    ClaudeProvider::with_clock(config, fixed_now).unwrap()
}

#[tokio::test]
async fn an_expired_headroom_sign_in_is_refreshed_and_saved() {
    let root = tempfile::tempdir().unwrap();
    let server = server(1).await;
    let provider = provider(root.path(), &server);
    let home = provider.config.headroom_accounts_dir().join("h1");
    sign_in(&home, &home.join(".claude.json"));
    let account = provider.account_at(&home).await.unwrap().unwrap();
    let snapshot = provider.fetch_limits(&account).await.unwrap();
    assert_eq!(snapshot.source, LimitsSource::Live);
    let stored: Value =
        serde_json::from_slice(&fs::read(home.join(".credentials.json")).unwrap()).unwrap();
    assert_eq!(stored["claudeAiOauth"]["accessToken"], "fresh-token");
    assert_eq!(stored["claudeAiOauth"]["refreshToken"], "fresh-refresh");
}

#[tokio::test]
async fn an_expired_cli_sign_in_is_never_refreshed_or_rewritten() {
    let root = tempfile::tempdir().unwrap();
    let server = server(0).await;
    let provider = provider(root.path(), &server);
    let cli = root.path().join("home/.claude");
    sign_in(&cli, &root.path().join("home/.claude.json"));
    let file = cli.join(".credentials.json");
    let before = fs::read(&file).unwrap();
    let accounts = provider.discover().await.unwrap();
    assert_eq!(accounts[0].owner, CredentialOwner::Cli);
    let result = provider.fetch_limits(&accounts[0]).await;
    assert_eq!(result.unwrap_err(), ProviderError::SignInExpired);
    assert_eq!(fs::read(&file).unwrap(), before);
    assert!(!cli.join(".credentials.json.lock").exists());
}
