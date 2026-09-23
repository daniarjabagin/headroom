use wiremock::matchers::{body_json, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::*;
use crate::antigravity::raw::RawUserStatusEnvelope;

const SUMMARY: &str = include_str!("fixtures/quota_summary_cloud.json");
const USER_STATUS: &str = include_str!("fixtures/user_status.json");

fn token() -> SecretString {
    SecretString::new("ya29.fake".into())
}

async fn cloud(status: u16, body: &str) -> MockServer {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(SUMMARY_PATH))
        .and(header("authorization", "Bearer ya29.fake"))
        .and(header("user-agent", "antigravity"))
        .respond_with(ResponseTemplate::new(status).set_body_string(body))
        .mount(&server)
        .await;
    server
}

fn client(servers: &[&MockServer]) -> CloudClient {
    let bases: Vec<String> = servers.iter().map(|server| server.uri()).collect();
    CloudClient::new(Client::new(), &bases)
}

#[tokio::test]
async fn a_failing_first_base_falls_back_to_the_second() {
    let daily = cloud(503, "").await;
    let prod = cloud(200, SUMMARY).await;
    let summary = client(&[&daily, &prod])
        .quota_summary(&token())
        .await
        .unwrap();
    assert!(summary.groups().is_some());
}

#[tokio::test]
async fn auth_and_rate_limits_stop_at_the_first_base() {
    for (status, expected) in [
        (401, ProviderError::SignInExpired),
        (403, ProviderError::SignInExpired),
        (429, ProviderError::RateLimited { retry_after: None }),
    ] {
        let daily = cloud(status, "").await;
        let prod = cloud(200, SUMMARY).await;
        let result = client(&[&daily, &prod]).quota_summary(&token()).await;
        assert_eq!(result.unwrap_err(), expected, "{status}");
        assert!(prod.received_requests().await.unwrap().is_empty());
    }
}

#[tokio::test]
async fn when_every_base_fails_the_last_error_is_kept() {
    let daily = cloud(404, "").await;
    let prod = cloud(500, "").await;
    let result = client(&[&daily, &prod]).quota_summary(&token()).await;
    assert!(matches!(result, Err(ProviderError::Network(ref text)) if text.contains("500")));
}

#[tokio::test]
async fn a_malformed_body_is_an_invalid_response() {
    let prod = cloud(200, "<html>").await;
    let result = client(&[&prod]).quota_summary(&token()).await;
    assert!(matches!(result, Err(ProviderError::InvalidResponse(_))));
}

#[tokio::test]
async fn language_server_calls_send_csrf_and_metadata() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(format!("/{LS_SERVICE}/GetUserStatus")))
        .and(header("x-codeium-csrf-token", "csrf-1"))
        .and(header("connect-protocol-version", "1"))
        .and(body_json(json!({ "metadata": {
            "ideName": "antigravity", "extensionName": "antigravity",
            "ideVersion": "unknown", "locale": "en"
        }})))
        .respond_with(ResponseTemplate::new(200).set_body_string(USER_STATUS))
        .mount(&server)
        .await;
    let ls = LanguageServerClient::new().unwrap();
    let status: Option<RawUserStatusEnvelope> =
        ls.call(&server.uri(), "csrf-1", "GetUserStatus").await;
    assert!(status.unwrap().user_status.is_some());
    let refused: Option<RawUserStatusEnvelope> =
        ls.call(&server.uri(), "wrong", "GetUserStatus").await;
    assert!(refused.is_none());
}

#[test]
fn statuses_map_like_every_provider() {
    assert!(matches!(
        error_for(StatusCode::NOT_FOUND, None, SUMMARY_PATH),
        ProviderError::InvalidResponse(_)
    ));
}
