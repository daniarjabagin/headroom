use std::os::unix::fs::PermissionsExt;

use serde_json::json;
use wiremock::matchers::{body_json, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::*;

fn now() -> Timestamp {
    "2026-09-23T10:00:00Z".parse().unwrap()
}

fn expired_ms() -> i64 {
    now().as_millisecond() - 60_000
}

fn document() -> Value {
    json!({
        "claudeAiOauth": {
            "accessToken": "old-access",
            "refreshToken": "old-refresh",
            "expiresAt": expired_ms(),
            "scopes": ["user:inference", "user:profile"],
            "subscriptionType": "max",
            "rateLimitTier": "default_claude_max_20x"
        },
        "mcpOAuth": { "keep": true }
    })
}

fn home_with(document: &Value) -> tempfile::TempDir {
    let home = tempfile::tempdir().unwrap();
    fs::write(home.path().join(CREDENTIALS_FILE), document.to_string()).unwrap();
    home
}

fn stale() -> Credentials {
    parse_credentials(&document().to_string()).unwrap()
}

async fn token_server(status: u16, body: Value, calls: u64) -> MockServer {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/oauth/token"))
        .and(body_json(json!({
            "grant_type": "refresh_token",
            "refresh_token": "old-refresh",
            "client_id": "9d1c250a-e61b-44d9-88ed-5944d1962f5e",
            "scope": "user:inference user:profile"
        })))
        .respond_with(ResponseTemplate::new(status).set_body_json(body))
        .expect(calls)
        .mount(&server)
        .await;
    server
}

fn client(server: &MockServer) -> TokenClient {
    TokenClient::new(
        reqwest::Client::new(),
        &format!("{}/v1/oauth/token", server.uri()),
    )
}

fn granted() -> RawTokens {
    RawTokens {
        access_token: Some(" new-access ".into()),
        refresh_token: Some("new-refresh".into()),
        expires_in: Some(28_800),
        scope: Some("user:inference user:profile user:sessions:claude_code".into()),
    }
}

#[test]
fn patching_replaces_tokens_expiry_and_scopes_only() {
    let patched = patch(document(), &granted(), now()).unwrap();
    let oauth = &patched["claudeAiOauth"];
    assert_eq!(oauth["accessToken"], "new-access");
    assert_eq!(oauth["refreshToken"], "new-refresh");
    assert_eq!(oauth["expiresAt"], now().as_millisecond() + 28_800_000);
    assert_eq!(
        oauth["scopes"],
        json!([
            "user:inference",
            "user:profile",
            "user:sessions:claude_code"
        ])
    );
    assert_eq!(oauth["subscriptionType"], "max");
    assert_eq!(oauth["rateLimitTier"], "default_claude_max_20x");
    assert_eq!(patched["mcpOAuth"], json!({ "keep": true }));
}

#[test]
fn a_response_without_a_refresh_token_keeps_the_old_one() {
    let partial = RawTokens {
        refresh_token: None,
        scope: None,
        ..granted()
    };
    let patched = patch(document(), &partial, now()).unwrap();
    assert_eq!(patched["claudeAiOauth"]["refreshToken"], "old-refresh");
    assert_eq!(
        patched["claudeAiOauth"]["scopes"],
        json!(["user:inference", "user:profile"])
    );
}

#[test]
fn a_response_without_an_access_token_is_refused() {
    assert!(matches!(
        patch(document(), &RawTokens::default(), now()),
        Err(ProviderError::InvalidResponse(_))
    ));
    assert!(matches!(
        patch(json!({}), &granted(), now()),
        Err(ProviderError::LocalData(_))
    ));
}

#[tokio::test]
async fn a_refresh_rewrites_the_headroom_file_atomically() {
    let body = json!({
        "access_token": "new-access",
        "refresh_token": "new-refresh",
        "expires_in": 28_800,
        "scope": "user:inference user:profile"
    });
    let server = token_server(200, body, 1).await;
    let home = home_with(&document());
    let fresh = refresh(&client(&server), home.path(), &stale(), now())
        .await
        .unwrap();
    assert_eq!(fresh.token_secret(), "new-access");
    assert!(fresh.usable_token(now()).is_ok());
    let file = home.path().join(CREDENTIALS_FILE);
    let stored: Value = serde_json::from_slice(&fs::read(&file).unwrap()).unwrap();
    assert_eq!(stored["claudeAiOauth"]["refreshToken"], "new-refresh");
    assert_eq!(stored["mcpOAuth"], json!({ "keep": true }));
    assert_eq!(
        fs::metadata(&file).unwrap().permissions().mode() & 0o777,
        0o600
    );
}

#[tokio::test]
async fn a_rejected_refresh_leaves_the_file_alone() {
    let server = token_server(400, json!({ "error": "invalid_grant" }), 1).await;
    let home = home_with(&document());
    let file = home.path().join(CREDENTIALS_FILE);
    let before = fs::read(&file).unwrap();
    let result = refresh(&client(&server), home.path(), &stale(), now()).await;
    assert_eq!(result.unwrap_err(), ProviderError::SignInExpired);
    assert_eq!(fs::read(&file).unwrap(), before);
}

#[tokio::test]
async fn a_token_refreshed_meanwhile_is_reused_without_a_request() {
    let server = token_server(200, json!({}), 0).await;
    let mut current = document();
    current["claudeAiOauth"]["accessToken"] = json!("other-access");
    current["claudeAiOauth"]["expiresAt"] = json!(now().as_millisecond() + 3_600_000);
    let home = home_with(&current);
    let fresh = refresh(&client(&server), home.path(), &stale(), now())
        .await
        .unwrap();
    assert_eq!(fresh.token_secret(), "other-access");
}

#[tokio::test]
async fn a_held_lock_blocks_a_second_refresh() {
    let home = home_with(&document());
    let _held = lock_credentials(home.path()).unwrap();
    let client = TokenClient::new(reqwest::Client::new(), "http://127.0.0.1:9");
    let result = refresh(&client, home.path(), &stale(), now()).await;
    assert!(matches!(result, Err(ProviderError::LocalData(_))));
}

#[tokio::test]
async fn a_sign_in_without_a_refresh_token_needs_a_new_sign_in() {
    let mut without = document();
    without["claudeAiOauth"]
        .as_object_mut()
        .unwrap()
        .remove("refreshToken");
    let home = home_with(&without);
    let client = TokenClient::new(reqwest::Client::new(), "http://127.0.0.1:9");
    let result = refresh(&client, home.path(), &stale(), now()).await;
    assert_eq!(result.unwrap_err(), ProviderError::SignInExpired);
}

async fn racing_token_server(file: std::path::PathBuf, concurrent: Value) -> MockServer {
    let server = MockServer::start().await;
    let body = json!({
        "access_token": "new-access",
        "refresh_token": "new-refresh",
        "expires_in": 28_800,
        "scope": "user:inference user:profile"
    });
    Mock::given(method("POST"))
        .and(path("/v1/oauth/token"))
        .respond_with(move |_: &wiremock::Request| {
            fs::write(&file, concurrent.to_string()).unwrap();
            ResponseTemplate::new(200).set_body_json(body.clone())
        })
        .expect(1)
        .mount(&server)
        .await;
    server
}

fn stored_oauth(home: &tempfile::TempDir) -> Value {
    let bytes = fs::read(home.path().join(CREDENTIALS_FILE)).unwrap();
    serde_json::from_slice::<Value>(&bytes).unwrap()["claudeAiOauth"].clone()
}

#[tokio::test]
async fn a_new_sign_in_written_during_the_refresh_is_adopted() {
    let home = home_with(&document());
    let mut signed_in = document();
    signed_in["claudeAiOauth"]["accessToken"] = json!("other-access");
    signed_in["claudeAiOauth"]["refreshToken"] = json!("other-refresh");
    signed_in["claudeAiOauth"]["expiresAt"] = json!(now().as_millisecond() + 3_600_000);
    let file = home.path().join(CREDENTIALS_FILE);
    let server = racing_token_server(file, signed_in).await;
    let fresh = refresh(&client(&server), home.path(), &stale(), now())
        .await
        .unwrap();
    assert_eq!(fresh.token_secret(), "other-access");
    assert_eq!(stored_oauth(&home)["accessToken"], "other-access");
    assert_eq!(stored_oauth(&home)["refreshToken"], "other-refresh");
}

#[tokio::test]
async fn an_expired_change_during_the_refresh_keeps_the_rotated_tokens() {
    let home = home_with(&document());
    let mut touched = document();
    touched["mcpOAuth"] = json!({ "keep": false });
    let file = home.path().join(CREDENTIALS_FILE);
    let server = racing_token_server(file, touched).await;
    let fresh = refresh(&client(&server), home.path(), &stale(), now())
        .await
        .unwrap();
    assert_eq!(fresh.token_secret(), "new-access");
    assert_eq!(stored_oauth(&home)["accessToken"], "new-access");
    assert_eq!(stored_oauth(&home)["refreshToken"], "new-refresh");
}
