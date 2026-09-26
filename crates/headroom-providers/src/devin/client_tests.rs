use headroom_core::secret::SecretString;
use jiff::SignedDuration;
use serde_json::json;
use wiremock::matchers::{body_json, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::*;

const FULL: &str = include_str!("fixtures/user_status.json");
const UNAUTHENTICATED: &str = include_str!("fixtures/error_unauthenticated.json");

fn now() -> Timestamp {
    "2026-09-23T10:00:00Z".parse().unwrap()
}

fn key() -> DevinKey {
    DevinKey {
        api_key: SecretString::new("fake-key".into()),
        api_server: None,
    }
}

async fn answering(status: u16, body: &str, retry_after: Option<&str>) -> MockServer {
    let server = MockServer::start().await;
    let mut response = ResponseTemplate::new(status).set_body_string(body);
    if let Some(value) = retry_after {
        response = response.insert_header("retry-after", value);
    }
    Mock::given(method("POST"))
        .and(path(STATUS_PATH))
        .respond_with(response)
        .mount(&server)
        .await;
    server
}

async fn fetch(server: &MockServer) -> Result<RawStatusResponse, ProviderError> {
    let client = StatusClient::new(crate::http::client().unwrap());
    client.fetch(&server.uri(), &key(), now()).await
}

#[tokio::test]
async fn the_key_travels_in_the_connect_request_body() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(STATUS_PATH))
        .and(header("connect-protocol-version", "1"))
        .and(header("content-type", "application/json"))
        .and(body_json(json!({ "metadata": {
            "apiKey": "fake-key",
            "ideName": "devin",
            "ideVersion": "1.108.2",
            "extensionName": "devin",
            "extensionVersion": "1.108.2",
            "locale": "en"
        }})))
        .respond_with(ResponseTemplate::new(200).set_body_string(FULL))
        .expect(1)
        .mount(&server)
        .await;
    let raw = fetch(&server).await.unwrap();
    assert_eq!(raw.user_status.email.as_deref(), Some("user@example.test"));
}

#[tokio::test]
async fn a_rejected_key_is_an_expired_sign_in() {
    for status in [401, 403] {
        let server = answering(status, UNAUTHENTICATED, None).await;
        assert_eq!(
            fetch(&server).await.unwrap_err(),
            ProviderError::SignInExpired
        );
    }
}

#[tokio::test]
async fn rate_limits_carry_the_retry_delay() {
    let server = answering(429, "{}", Some("120")).await;
    assert_eq!(
        fetch(&server).await.unwrap_err(),
        ProviderError::rate_limited(Some(SignedDuration::from_secs(120)))
    );
    let dated = answering(429, "{}", Some("Wed, 23 Sep 2026 10:05:00 GMT")).await;
    assert_eq!(
        fetch(&dated).await.unwrap_err(),
        ProviderError::rate_limited(Some(SignedDuration::from_mins(5)))
    );
}

#[tokio::test]
async fn plan_errors_server_errors_and_garbage_are_told_apart() {
    let plan = answering(
        403,
        r#"{"code":"permission_denied","message":"no active plan"}"#,
        None,
    );
    assert!(matches!(
        fetch(&plan.await).await,
        Err(ProviderError::NoSubscription { .. })
    ));
    let down = answering(503, "", None).await;
    assert!(matches!(fetch(&down).await, Err(ProviderError::Network(_))));
    let odd = answering(404, "", None).await;
    assert!(matches!(
        fetch(&odd).await,
        Err(ProviderError::InvalidResponse(_))
    ));
    let garbage = answering(200, "<html>", None).await;
    assert!(matches!(
        fetch(&garbage).await,
        Err(ProviderError::InvalidResponse(_))
    ));
}
