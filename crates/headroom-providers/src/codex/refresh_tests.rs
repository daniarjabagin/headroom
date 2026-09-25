use std::fs;
use std::os::unix::fs::PermissionsExt;

use serde_json::json;
use wiremock::matchers::{body_json, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::*;
use crate::codex::auth::load_credentials;
use crate::codex::test_support::{NOW, access_token, at, auth_document, id_token, write_auth};

fn tokens() -> RawTokens {
    RawTokens {
        id: Some("new-id".into()),
        access: Some(" new-access ".into()),
        refresh: None,
    }
}

async fn token_server(status: u16, body: serde_json::Value, calls: u64) -> MockServer {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/oauth/token"))
        .and(body_json(json!({
            "client_id": "app_EMoamEEZ73f0CkXaXp7hrann",
            "grant_type": "refresh_token",
            "refresh_token": "rt-fake"
        })))
        .respond_with(ResponseTemplate::new(status).set_body_json(body))
        .expect(calls)
        .mount(&server)
        .await;
    server
}

fn expired_home() -> tempfile::TempDir {
    let home = tempfile::tempdir().unwrap();
    let document = auth_document(&access_token(at("2026-09-23T09:00:00Z")));
    write_auth(home.path(), &document);
    home
}

#[test]
fn patching_replaces_returned_tokens_and_stamps_the_refresh() {
    let document = auth_document("old-access");
    let patched = patch(document, &tokens(), at(NOW)).unwrap();
    assert_eq!(patched["tokens"]["access_token"], "new-access");
    assert_eq!(patched["tokens"]["id_token"], "new-id");
    assert_eq!(patched["tokens"]["refresh_token"], "rt-fake");
    assert_eq!(patched["tokens"]["account_id"], "acct-fake0001");
    assert_eq!(patched["last_refresh"], NOW);
    assert_eq!(patched["OPENAI_API_KEY"], json!(null));
}

#[test]
fn a_response_without_an_access_token_is_refused() {
    let empty = RawTokens::default();
    assert!(matches!(
        patch(auth_document("a"), &empty, at(NOW)),
        Err(ProviderError::InvalidResponse(_))
    ));
    assert!(matches!(
        patch(json!({}), &tokens(), at(NOW)),
        Err(ProviderError::LocalData(_))
    ));
}

#[tokio::test]
async fn a_refresh_rewrites_the_headroom_file_atomically() {
    let fresh_access = access_token(at("2026-09-23T20:00:00Z"));
    let body =
        json!({ "access_token": fresh_access, "id_token": id_token(), "refresh_token": "rt-new" });
    let server = token_server(200, body, 1).await;
    let home = expired_home();
    let stale = load_credentials(home.path()).unwrap();
    let client = TokenClient::new(reqwest::Client::new(), &server.uri());
    let fresh = refresh(&client, home.path(), &stale, at(NOW))
        .await
        .unwrap();
    assert_eq!(fresh.access_token, fresh_access);
    assert_eq!(fresh.identity, stale.identity);
    let file = home.path().join(AUTH_FILE);
    let stored: Value = serde_json::from_slice(&fs::read(&file).unwrap()).unwrap();
    assert_eq!(stored["tokens"]["refresh_token"], "rt-new");
    assert_eq!(stored["last_refresh"], NOW);
    assert_eq!(
        fs::metadata(&file).unwrap().permissions().mode() & 0o777,
        0o600
    );
    let names: Vec<_> = fs::read_dir(home.path()).unwrap().collect();
    assert_eq!(names.len(), 2);
}

#[tokio::test]
async fn a_rejected_refresh_leaves_the_file_alone() {
    let server = token_server(401, json!({ "error": "refresh_token_expired" }), 1).await;
    let home = expired_home();
    let before = fs::read(home.path().join(AUTH_FILE)).unwrap();
    let stale = load_credentials(home.path()).unwrap();
    let client = TokenClient::new(reqwest::Client::new(), &server.uri());
    let result = refresh(&client, home.path(), &stale, at(NOW)).await;
    assert_eq!(result, Err(ProviderError::SignInExpired));
    assert_eq!(fs::read(home.path().join(AUTH_FILE)).unwrap(), before);
}

#[tokio::test]
async fn a_token_refreshed_meanwhile_is_reused_without_a_request() {
    let server = token_server(200, json!({}), 0).await;
    let home = expired_home();
    let stale = load_credentials(home.path()).unwrap();
    let current = auth_document(&access_token(at("2026-09-23T20:00:00Z")));
    write_auth(home.path(), &current);
    let client = TokenClient::new(reqwest::Client::new(), &server.uri());
    let fresh = refresh(&client, home.path(), &stale, at(NOW))
        .await
        .unwrap();
    assert_ne!(fresh.access_token, stale.access_token);
}

#[tokio::test]
async fn a_held_lock_blocks_a_second_refresh() {
    let home = expired_home();
    let _held = lock_auth(home.path()).unwrap();
    let stale = load_credentials(home.path()).unwrap();
    let client = TokenClient::new(reqwest::Client::new(), "http://127.0.0.1:9");
    let result = refresh(&client, home.path(), &stale, at(NOW)).await;
    assert!(matches!(result, Err(ProviderError::LocalData(_))));
}

#[tokio::test]
async fn a_sign_in_without_a_refresh_token_needs_a_new_sign_in() {
    let home = tempfile::tempdir().unwrap();
    let mut document = auth_document(&access_token(at("2026-09-23T09:00:00Z")));
    document["tokens"]
        .as_object_mut()
        .unwrap()
        .remove("refresh_token");
    write_auth(home.path(), &document);
    let stale = load_credentials(home.path()).unwrap();
    let client = TokenClient::new(reqwest::Client::new(), "http://127.0.0.1:9");
    let result = refresh(&client, home.path(), &stale, at(NOW)).await;
    assert_eq!(result, Err(ProviderError::SignInExpired));
}

async fn racing_token_server(home: &Path, concurrent: Value, body: Value) -> MockServer {
    let server = MockServer::start().await;
    let home = home.to_path_buf();
    Mock::given(method("POST"))
        .and(path("/oauth/token"))
        .respond_with(move |_: &wiremock::Request| {
            write_auth(&home, &concurrent);
            ResponseTemplate::new(200).set_body_json(body.clone())
        })
        .expect(1)
        .mount(&server)
        .await;
    server
}

fn stored_tokens(home: &Path) -> Value {
    let bytes = fs::read(home.join(AUTH_FILE)).unwrap();
    serde_json::from_slice::<Value>(&bytes).unwrap()["tokens"].clone()
}

async fn refresh_racing(home: &Path, concurrent: Value, rotated_access: &str) -> Credentials {
    let stale = load_credentials(home).unwrap();
    let rotated = json!({ "access_token": rotated_access, "refresh_token": "rt-new" });
    let server = racing_token_server(home, concurrent, rotated).await;
    let client = TokenClient::new(reqwest::Client::new(), &server.uri());
    refresh(&client, home, &stale, at(NOW)).await.unwrap()
}

#[tokio::test]
async fn a_new_sign_in_written_during_the_refresh_is_adopted() {
    let home = expired_home();
    let signed_in_access = access_token(at("2026-09-23T21:00:00Z"));
    let mut signed_in = auth_document(&signed_in_access);
    signed_in["tokens"]["refresh_token"] = json!("rt-other");
    let rotated_access = access_token(at("2026-09-23T20:00:00Z"));
    let fresh = refresh_racing(home.path(), signed_in, &rotated_access).await;
    assert_eq!(fresh.access_token, signed_in_access);
    assert_eq!(stored_tokens(home.path())["access_token"], signed_in_access);
    assert_eq!(stored_tokens(home.path())["refresh_token"], "rt-other");
}

#[tokio::test]
async fn an_expired_change_during_the_refresh_keeps_the_rotated_tokens() {
    let home = expired_home();
    let mut touched = auth_document(&access_token(at("2026-09-23T09:00:00Z")));
    touched["last_refresh"] = json!("2026-09-23T09:59:00Z");
    let rotated_access = access_token(at("2026-09-23T20:00:00Z"));
    let fresh = refresh_racing(home.path(), touched, &rotated_access).await;
    assert_eq!(fresh.access_token, rotated_access);
    assert_eq!(stored_tokens(home.path())["access_token"], rotated_access);
    assert_eq!(stored_tokens(home.path())["refresh_token"], "rt-new");
}
