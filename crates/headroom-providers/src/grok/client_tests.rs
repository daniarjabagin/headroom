use jiff::SignedDuration;
use wiremock::matchers::{body_string, header, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::*;

const WEEKLY: &str = include_str!("fixtures/billing_weekly.json");
const SETTINGS: &str = include_str!("fixtures/settings.json");

fn client(server: &MockServer) -> GrokClient {
    GrokClient::new(reqwest::Client::new(), &format!("{}/v1/", server.uri()))
}

async fn respond(status: u16, body: &str, retry_after: Option<&str>) -> MockServer {
    let server = MockServer::start().await;
    let mut response = ResponseTemplate::new(status).set_body_string(body);
    if let Some(value) = retry_after {
        response = response.insert_header("Retry-After", value);
    }
    Mock::given(method("GET"))
        .respond_with(response)
        .mount(&server)
        .await;
    server
}

#[tokio::test]
async fn billing_sends_the_cli_headers() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/billing"))
        .and(query_param("format", "credits"))
        .and(header("authorization", "Bearer fake-token"))
        .and(header("x-xai-token-auth", "xai-grok-cli"))
        .and(header("accept", "application/json"))
        .respond_with(ResponseTemplate::new(200).set_body_string(WEEKLY))
        .expect(1)
        .mount(&server)
        .await;
    let billing = client(&server).billing("fake-token").await.unwrap();
    assert_eq!(billing.config.credit_usage_percent, Some(37.5));
}

#[tokio::test]
async fn settings_carry_the_plan_name() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/settings"))
        .respond_with(ResponseTemplate::new(200).set_body_string(SETTINGS))
        .mount(&server)
        .await;
    let settings = client(&server).settings("fake-token").await.unwrap();
    assert_eq!(
        settings.subscription_tier_display.as_deref(),
        Some("SuperGrok")
    );
}

#[tokio::test]
async fn statuses_map_to_provider_errors() {
    let cases = [
        (401, "{}", ProviderError::SignInExpired),
        (
            403,
            r#"{"error":"token revoked"}"#,
            ProviderError::SignInExpired,
        ),
        (
            403,
            r#"{"error":"No active subscription"}"#,
            no_subscription(),
        ),
        (402, "", no_subscription()),
        (
            500,
            "",
            ProviderError::Network(
                "billing request returned HTTP 500 Internal Server Error".into(),
            ),
        ),
    ];
    for (status, body, expected) in cases {
        let server = respond(status, body, None).await;
        assert_eq!(
            client(&server).billing("t").await,
            Err(expected),
            "{status}"
        );
    }
}

#[tokio::test]
async fn rate_limits_keep_retry_after() {
    let server = respond(429, "", Some("120")).await;
    assert_eq!(
        client(&server).billing("t").await,
        Err(ProviderError::RateLimited {
            retry_after: Some(SignedDuration::from_secs(120))
        })
    );
}

#[tokio::test]
async fn unexpected_bodies_are_invalid_responses() {
    let server = respond(200, "{\"other\":1}", None).await;
    assert!(matches!(
        client(&server).billing("t").await,
        Err(ProviderError::InvalidResponse(_))
    ));
}

#[tokio::test]
async fn refresh_posts_an_encoded_form() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/oauth2/token"))
        .and(header("content-type", "application/x-www-form-urlencoded"))
        .and(body_string(
            "grant_type=refresh_token&client_id=client%201&refresh_token=a%2Bb%2Fc%3D",
        ))
        .respond_with(
            ResponseTemplate::new(200).set_body_string(
                r#"{"access_token":"new","refresh_token":"next","expires_in":21600}"#,
            ),
        )
        .expect(1)
        .mount(&server)
        .await;
    let tokens = client(&server)
        .refresh(&format!("{}/", server.uri()), "client 1", "a+b/c=")
        .await
        .unwrap();
    assert_eq!(tokens.access_token, "new");
    assert_eq!(tokens.refresh_token.as_deref(), Some("next"));
    assert_eq!(tokens.expires_in, Some(21_600));
}

#[tokio::test]
async fn a_rejected_refresh_means_signing_in_again() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(400).set_body_string(r#"{"error":"invalid_grant"}"#))
        .mount(&server)
        .await;
    assert!(matches!(
        client(&server).refresh(&server.uri(), "c", "r").await,
        Err(ProviderError::SignInExpired)
    ));
}
