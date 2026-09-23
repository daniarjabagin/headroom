use std::os::unix::fs::PermissionsExt;

use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::*;

const ENTRY: &str = "issuer::client-1";

fn now() -> Timestamp {
    "2026-09-23T10:00:00Z".parse().unwrap()
}

fn tokens(expires_in: Option<i64>) -> RawTokens {
    RawTokens {
        access_token: " new-access ".into(),
        refresh_token: Some("new-refresh".into()),
        id_token: None,
        expires_in,
    }
}

fn document(issuer: &str) -> Value {
    json!({
        ENTRY: {
            "key": "old-access",
            "refresh_token": "old-refresh",
            "oidc_client_id": "client-1",
            "oidc_issuer": issuer,
            "expires_at": "2026-09-23T09:00:00Z",
            "user_id": "user-1",
            "team_id": "team-1",
            "email": "ada@example.com"
        },
        "other": { "keep": true }
    })
}

fn home_with(issuer: &str) -> tempfile::TempDir {
    let home = tempfile::tempdir().unwrap();
    fs::write(home.path().join(AUTH_FILE), document(issuer).to_string()).unwrap();
    home
}

async fn token_server(status: u16, body: &str, calls: u64) -> MockServer {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/oauth2/token"))
        .respond_with(ResponseTemplate::new(status).set_body_string(body))
        .expect(calls)
        .mount(&server)
        .await;
    server
}

fn grok_client() -> GrokClient {
    GrokClient::new(reqwest::Client::new(), "http://127.0.0.1:9")
}

#[test]
fn patching_replaces_only_this_entrys_tokens() {
    let patched = patch(document("i"), ENTRY, &tokens(Some(3_600)), now()).unwrap();
    let entry = &patched[ENTRY];
    assert_eq!(entry["key"], "new-access");
    assert_eq!(entry["refresh_token"], "new-refresh");
    assert_eq!(entry["expires_at"], "2026-09-23T11:00:00Z");
    assert_eq!(entry["email"], "ada@example.com");
    assert!(entry.get("id_token").is_none());
    assert_eq!(patched["other"], json!({ "keep": true }));
}

#[test]
fn an_unknown_lifetime_drops_the_stale_expiry() {
    let patched = patch(document("i"), ENTRY, &tokens(None), now()).unwrap();
    assert!(patched[ENTRY].get("expires_at").is_none());
    let credentials = credentials_from(&patched).unwrap();
    assert_eq!(credentials.expires_at, None);
}

#[test]
fn an_empty_access_token_or_missing_entry_is_refused() {
    let empty = RawTokens {
        access_token: " ".into(),
        ..tokens(None)
    };
    assert!(matches!(
        patch(document("i"), ENTRY, &empty, now()),
        Err(ProviderError::InvalidResponse(_))
    ));
    assert!(matches!(
        patch(json!({}), ENTRY, &tokens(None), now()),
        Err(ProviderError::LocalData(_))
    ));
}

#[tokio::test]
async fn a_refresh_rewrites_the_headroom_file_atomically() {
    let server = token_server(
        200,
        r#"{"access_token":"new-access","refresh_token":"new-refresh","expires_in":21600}"#,
        1,
    )
    .await;
    let home = home_with(&server.uri());
    let stale = credentials_from(&document(&server.uri())).unwrap();
    let fresh = refresh(&grok_client(), &server.uri(), home.path(), &stale, now())
        .await
        .unwrap();
    assert_eq!(fresh.access_token, "new-access");
    assert_eq!(fresh.identity, stale.identity);
    let on_disk = load_credentials_at(home.path());
    assert_eq!(on_disk, fresh);
    let mode = fs::metadata(home.path().join(AUTH_FILE))
        .unwrap()
        .permissions()
        .mode();
    assert_eq!(mode & 0o777, 0o600);
}

#[tokio::test]
async fn a_rejected_refresh_leaves_the_file_alone() {
    let server = token_server(400, r#"{"error":"invalid_grant"}"#, 1).await;
    let home = home_with(&server.uri());
    let before = fs::read(home.path().join(AUTH_FILE)).unwrap();
    let stale = credentials_from(&document(&server.uri())).unwrap();
    let result = refresh(&grok_client(), &server.uri(), home.path(), &stale, now()).await;
    assert_eq!(result, Err(ProviderError::SignInExpired));
    assert_eq!(fs::read(home.path().join(AUTH_FILE)).unwrap(), before);
}

#[tokio::test]
async fn tokens_from_another_issuer_are_never_sent() {
    let server = token_server(200, "{}", 0).await;
    let home = home_with("https://sso.example.com");
    let stale = credentials_from(&document("https://sso.example.com")).unwrap();
    let result = refresh(&grok_client(), &server.uri(), home.path(), &stale, now()).await;
    assert_eq!(result, Err(ProviderError::SignInExpired));
}

#[tokio::test]
async fn a_token_refreshed_meanwhile_is_reused() {
    let server = token_server(200, "{}", 0).await;
    let home = home_with(&server.uri());
    let mut stale_credentials = credentials_from(&document(&server.uri())).unwrap();
    stale_credentials.access_token = "older-access".into();
    let mut current = document(&server.uri());
    current[ENTRY]["expires_at"] = json!("2026-09-23T18:00:00Z");
    fs::write(home.path().join(AUTH_FILE), current.to_string()).unwrap();
    let fresh = refresh(
        &grok_client(),
        &server.uri(),
        home.path(),
        &stale_credentials,
        now(),
    )
    .await
    .unwrap();
    assert_eq!(fresh.access_token, "old-access");
}

#[tokio::test]
async fn a_held_lock_blocks_a_second_refresh() {
    let home = home_with("issuer");
    let _held = lock_auth(home.path()).unwrap();
    let stale = credentials_from(&document("issuer")).unwrap();
    let result = refresh(&grok_client(), "issuer", home.path(), &stale, now()).await;
    assert!(matches!(result, Err(ProviderError::LocalData(_))));
}

fn load_credentials_at(home: &Path) -> Credentials {
    credentials_from(&read_auth_file(home).unwrap().document).unwrap()
}
