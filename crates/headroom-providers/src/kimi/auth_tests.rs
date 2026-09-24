use std::fs;
use std::time::Duration;

use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::*;
use crate::kimi::credentials::CREDENTIALS_FILE;

const REFRESHED: &str = include_str!("fixtures/refreshed.json");

fn now() -> Timestamp {
    "2026-09-23T10:00:00Z".parse().unwrap()
}

fn home_with(expires_at: Timestamp) -> tempfile::TempDir {
    let home = tempfile::tempdir().unwrap();
    let path = home.path().join(CREDENTIALS_FILE);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    let tokens = json!({
        "access_token": "old-access",
        "refresh_token": "old-refresh",
        "expires_at": expires_at.as_second()
    });
    fs::write(path, tokens.to_string()).unwrap();
    home
}

async fn token_server(delay: Duration, calls: u64) -> MockServer {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/oauth/token"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(REFRESHED)
                .set_delay(delay),
        )
        .expect(calls)
        .mount(&server)
        .await;
    server
}

fn kimi_client(server: &MockServer) -> KimiClient {
    KimiClient::new(reqwest::Client::new(), &server.uri(), &server.uri())
}

#[tokio::test]
async fn a_refresh_saves_the_rotated_tokens() {
    let server = token_server(Duration::ZERO, 1).await;
    let home = home_with(now());
    let token = refresh(&kimi_client(&server), home.path(), now())
        .await
        .unwrap();
    assert_eq!(token.expose(), "fake-access-token-2");
    let saved = credentials::load(home.path()).unwrap();
    assert_eq!(saved.refresh.expose(), "fake-refresh-token-2");
}

#[tokio::test]
async fn a_token_refreshed_meanwhile_is_reused() {
    let server = token_server(Duration::ZERO, 0).await;
    let home = home_with(now() + SignedDuration::from_hours(1));
    let token = refresh(&kimi_client(&server), home.path(), now())
        .await
        .unwrap();
    assert_eq!(token.expose(), "old-access");
}

#[tokio::test]
async fn a_held_lock_blocks_a_second_refresh() {
    let server = token_server(Duration::ZERO, 0).await;
    let home = home_with(now());
    let _held = lock_credentials(home.path()).unwrap();
    let result = refresh(&kimi_client(&server), home.path(), now()).await;
    assert!(matches!(result, Err(ProviderError::LocalData(_))));
}

#[tokio::test]
async fn a_file_changed_during_the_refresh_is_kept_and_the_token_still_used() {
    let server = token_server(Duration::from_millis(200), 1).await;
    let home = home_with(now());
    let path = home.path().join(CREDENTIALS_FILE);
    let client = kimi_client(&server);
    let pending = refresh(&client, home.path(), now());
    let rewrite = async {
        tokio::time::sleep(Duration::from_millis(50)).await;
        fs::write(&path, r#"{"access_token":"cli","refresh_token":"cli"}"#).unwrap();
    };
    let (token, ()) = tokio::join!(pending, rewrite);
    assert_eq!(token.unwrap().expose(), "fake-access-token-2");
    assert_eq!(
        credentials::load(home.path()).unwrap().access.expose(),
        "cli"
    );
}
