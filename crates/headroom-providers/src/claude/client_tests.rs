use jiff::SignedDuration;
use wiremock::matchers::{header, header_regex, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::super::auth::parse_credentials;
use super::*;

const FULL: &str = include_str!("fixtures/usage_full.json");
const FORBIDDEN_PLAN: &str = include_str!("fixtures/forbidden_plan.json");

fn now() -> Timestamp {
    "2026-09-23T10:00:00Z".parse().unwrap()
}

async fn server_responding(response: ResponseTemplate) -> MockServer {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/oauth/usage"))
        .respond_with(response)
        .mount(&server)
        .await;
    server
}

async fn fetch_at(base: &str) -> Result<RawUsage, ProviderError> {
    let client = UsageClient::new(crate::http::client().unwrap(), base);
    let credentials =
        parse_credentials(r#"{"claudeAiOauth":{"accessToken":"fake-token"}}"#).unwrap();
    let token = credentials.usable_token(now()).unwrap();
    client.fetch(token, now()).await
}

async fn fetch_from(server: &MockServer) -> Result<RawUsage, ProviderError> {
    fetch_at(&format!("{}/", server.uri())).await
}

#[tokio::test]
async fn success_sends_expected_headers_and_parses_body() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/oauth/usage"))
        .and(header("authorization", "Bearer fake-token"))
        .and(header("anthropic-beta", "oauth-2025-04-20"))
        .and(header("accept", "application/json"))
        .and(header_regex("user-agent", r"^headroom/\d+\.\d+\.\d+$"))
        .respond_with(ResponseTemplate::new(200).set_body_string(FULL))
        .expect(1)
        .mount(&server)
        .await;
    let raw = fetch_from(&server).await.unwrap();
    assert_eq!(raw.five_hour.unwrap().utilization, Some(37.0));
    assert_eq!(raw.limits.unwrap().len(), 3);
    assert!(raw.other.contains_key("seven_day_opus"));
}

#[tokio::test]
async fn unauthorized_and_forbidden_mean_sign_in_expired() {
    for status in [401, 403] {
        let server = server_responding(ResponseTemplate::new(status)).await;
        assert_eq!(
            fetch_from(&server).await.unwrap_err(),
            ProviderError::SignInExpired
        );
    }
}

#[tokio::test]
async fn plan_errors_mean_no_subscription() {
    let expected = ProviderError::NoSubscription {
        detail: "No active Claude subscription.".into(),
    };
    let payment = server_responding(ResponseTemplate::new(402)).await;
    assert_eq!(fetch_from(&payment).await.unwrap_err(), expected);
    for status in [403, 404] {
        let response = ResponseTemplate::new(status).set_body_string(FORBIDDEN_PLAN);
        let server = server_responding(response).await;
        assert_eq!(fetch_from(&server).await.unwrap_err(), expected);
    }
}

#[tokio::test]
async fn rate_limit_reads_retry_after_seconds() {
    let server =
        server_responding(ResponseTemplate::new(429).insert_header("retry-after", "120")).await;
    assert_eq!(
        fetch_from(&server).await.unwrap_err(),
        ProviderError::rate_limited(Some(SignedDuration::from_secs(120)))
    );
}

#[tokio::test]
async fn rate_limit_reads_retry_after_http_date() {
    let response =
        ResponseTemplate::new(429).insert_header("retry-after", "Wed, 23 Sep 2026 10:05:00 GMT");
    let server = server_responding(response).await;
    assert_eq!(
        fetch_from(&server).await.unwrap_err(),
        ProviderError::rate_limited(Some(SignedDuration::from_secs(300)))
    );
}

#[tokio::test]
async fn rate_limit_without_header_has_no_delay() {
    let server = server_responding(ResponseTemplate::new(429)).await;
    assert_eq!(
        fetch_from(&server).await.unwrap_err(),
        ProviderError::rate_limited(None)
    );
}

#[tokio::test]
async fn server_errors_are_network_and_client_errors_invalid() {
    let server = server_responding(ResponseTemplate::new(503)).await;
    assert!(matches!(
        fetch_from(&server).await.unwrap_err(),
        ProviderError::Network(message) if message.contains("503")
    ));
    let server = server_responding(ResponseTemplate::new(404)).await;
    assert!(matches!(
        fetch_from(&server).await.unwrap_err(),
        ProviderError::InvalidResponse(message) if message.contains("404")
    ));
}

#[tokio::test]
async fn garbage_body_is_invalid_response() {
    for body in [
        "<html>oops</html>",
        "[1,2]",
        "{\"five_hour\": {\"utilization\": \"x\"}}",
    ] {
        let server = server_responding(ResponseTemplate::new(200).set_body_string(body)).await;
        assert!(matches!(
            fetch_from(&server).await.unwrap_err(),
            ProviderError::InvalidResponse(_)
        ));
    }
}

#[tokio::test]
async fn unreachable_server_is_network_error_without_token() {
    let error = fetch_at("http://127.0.0.1:1").await.unwrap_err();
    assert!(matches!(&error, ProviderError::Network(_)));
    assert!(!error.to_string().contains("fake-token"));
}
