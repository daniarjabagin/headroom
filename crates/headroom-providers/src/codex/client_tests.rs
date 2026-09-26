use headroom_core::account::AccountIdentity;
use jiff::SignedDuration;
use wiremock::matchers::{header, header_exists, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::super::test_support::at;
use super::*;

const FULL: &str = include_str!("fixtures/usage_full.json");
const WEEKLY_ONLY: &str = include_str!("fixtures/usage_weekly_only.json");
const FORBIDDEN_PLAN: &str = include_str!("fixtures/forbidden_plan.json");
const NOW: &str = "2026-09-21T14:13:20Z";

fn credentials(account_id: Option<&str>) -> Credentials {
    Credentials {
        access_token: "token-fake".into(),
        account_id: account_id.map(str::to_owned),
        identity: AccountIdentity {
            email: None,
            plan: None,
            stable_key: "u/a".into(),
        },
        expires_at: None,
        signed_in_at: None,
    }
}

async fn serve(response: ResponseTemplate) -> MockServer {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/backend-api/wham/usage"))
        .respond_with(response)
        .mount(&server)
        .await;
    server
}

async fn fetch(
    server: &MockServer,
    account_id: Option<&str>,
) -> Result<UsageResponse, ProviderError> {
    let client = UsageClient::new(
        crate::http::client().unwrap(),
        &format!("{}/", server.uri()),
    );
    client.fetch_usage(&credentials(account_id), at(NOW)).await
}

fn json_body(body: &str) -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_raw(body.as_bytes().to_vec(), "application/json")
}

#[tokio::test]
async fn sends_expected_headers_and_parses_body() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/backend-api/wham/usage"))
        .and(header("authorization", "Bearer token-fake"))
        .and(header("accept", "application/json"))
        .and(header("chatgpt-account-id", "acct-1"))
        .and(header(
            "user-agent",
            format!("headroom/{}", env!("CARGO_PKG_VERSION")).as_str(),
        ))
        .respond_with(json_body(FULL))
        .expect(1)
        .mount(&server)
        .await;
    let response = fetch(&server, Some("acct-1")).await.unwrap();
    assert_eq!(response.plan_type.as_deref(), Some("plus"));
    assert_eq!(
        response.additional_rate_limits.map(|limits| limits.len()),
        Some(1)
    );
}

#[tokio::test]
async fn account_header_is_omitted_when_unknown() {
    let server = MockServer::start().await;
    Mock::given(header_exists("chatgpt-account-id"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .respond_with(json_body(WEEKLY_ONLY))
        .mount(&server)
        .await;
    let response = fetch(&server, None).await.unwrap();
    assert_eq!(response.plan_type.as_deref(), Some("prolite"));
}

#[tokio::test]
async fn numeric_strings_are_accepted() {
    let server = serve(json_body(WEEKLY_ONLY)).await;
    let response = fetch(&server, None).await.unwrap();
    let primary = response.rate_limit.unwrap().primary_window.unwrap();
    assert_eq!(primary.used_percent.unwrap().as_f64(), Some(28.0));
    assert_eq!(primary.limit_window_seconds.unwrap().whole(), Some(604_800));
}

#[tokio::test]
async fn unauthorized_and_forbidden_mean_sign_in_expired() {
    for status in [401, 403] {
        let server = serve(ResponseTemplate::new(status)).await;
        assert_eq!(
            fetch(&server, None).await,
            Err(ProviderError::SignInExpired)
        );
    }
}

#[tokio::test]
async fn payment_required_and_plan_forbidden_mean_no_subscription() {
    let expected = Err(ProviderError::NoSubscription {
        detail: "No active ChatGPT subscription.".into(),
    });
    let payment = serve(ResponseTemplate::new(402)).await;
    assert_eq!(fetch(&payment, None).await, expected);
    let forbidden = serve(
        ResponseTemplate::new(403)
            .set_body_raw(FORBIDDEN_PLAN.as_bytes().to_vec(), "application/json"),
    )
    .await;
    assert_eq!(fetch(&forbidden, None).await, expected);
}

#[tokio::test]
async fn forbidden_without_plan_wording_means_sign_in_expired() {
    let body = r#"{"detail":"Could not validate your token. Please sign in again."}"#;
    let server = serve(ResponseTemplate::new(403).set_body_raw(body, "application/json")).await;
    assert_eq!(
        fetch(&server, None).await,
        Err(ProviderError::SignInExpired)
    );
}

#[tokio::test]
async fn too_many_requests_honours_retry_after_seconds() {
    let server = serve(ResponseTemplate::new(429).insert_header("retry-after", "120")).await;
    assert_eq!(
        fetch(&server, None).await,
        Err(ProviderError::rate_limited(Some(
            SignedDuration::from_secs(120)
        )))
    );
}

#[tokio::test]
async fn too_many_requests_honours_retry_after_date() {
    let date = "Mon, 21 Sep 2026 14:18:20 GMT";
    let server = serve(ResponseTemplate::new(429).insert_header("retry-after", date)).await;
    assert_eq!(
        fetch(&server, None).await,
        Err(ProviderError::rate_limited(Some(
            SignedDuration::from_mins(5)
        )))
    );
}

#[tokio::test]
async fn too_many_requests_without_header_has_no_hint() {
    let server = serve(ResponseTemplate::new(429)).await;
    assert_eq!(
        fetch(&server, None).await,
        Err(ProviderError::rate_limited(None))
    );
}

#[tokio::test]
async fn server_errors_are_network_errors() {
    let server = serve(ResponseTemplate::new(502)).await;
    let Err(ProviderError::Network(message)) = fetch(&server, None).await else {
        panic!("expected a network error");
    };
    assert!(message.contains("502"));
    assert!(!message.contains("token-fake"));
}

#[tokio::test]
async fn garbage_body_is_invalid_response() {
    let server = serve(json_body("<html>oops</html>")).await;
    assert!(matches!(
        fetch(&server, None).await,
        Err(ProviderError::InvalidResponse(_))
    ));
    let array = serve(json_body("[1,2]")).await;
    assert!(matches!(
        fetch(&array, None).await,
        Err(ProviderError::InvalidResponse(_))
    ));
}

#[tokio::test]
async fn unreachable_server_is_network_error() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let uri = format!("http://{}", listener.local_addr().unwrap());
    drop(listener);
    let client = UsageClient::new(crate::http::client().unwrap(), &uri);
    let result = client.fetch_usage(&credentials(None), at(NOW)).await;
    assert!(matches!(result, Err(ProviderError::Network(_))));
}
