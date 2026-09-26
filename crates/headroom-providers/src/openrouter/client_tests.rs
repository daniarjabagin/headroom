use headroom_core::units::MicroUsd;
use jiff::SignedDuration;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::super::money::Usd;
use super::*;

const KEY: &str = include_str!("fixtures/key.json");
const CREDITS: &str = include_str!("fixtures/credits.json");
const CREDITS_FORBIDDEN: &str = include_str!("fixtures/credits_forbidden.json");
const INVALID_KEY: &str = include_str!("fixtures/invalid_key.json");
const RATE_LIMITED: &str = include_str!("fixtures/rate_limited.json");

fn now() -> Timestamp {
    "2026-09-23T10:00:00Z".parse().unwrap()
}

async fn serving(endpoint: &str, response: ResponseTemplate) -> (MockServer, KeyClient) {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(endpoint))
        .and(header("authorization", "Bearer sk-or-v1-test"))
        .and(header("accept", "application/json"))
        .respond_with(response)
        .mount(&server)
        .await;
    let client = KeyClient::new(
        crate::http::client().unwrap(),
        &format!("{}/", server.uri()),
    );
    (server, client)
}

fn json(status: u16, body: &str) -> ResponseTemplate {
    ResponseTemplate::new(status).set_body_string(body)
}

#[tokio::test]
async fn key_details_parse_with_bearer_auth() {
    let (_server, client) = serving("/api/v1/key", json(200, KEY)).await;
    let key = client.key("sk-or-v1-test", now()).await.unwrap();
    assert_eq!(key.usage_monthly, Some(Usd(MicroUsd(12_050_000))));
    assert_eq!(key.limit, None);
    assert_eq!(key.is_free_tier, Some(false));
}

#[tokio::test]
async fn an_invalid_key_means_sign_in_expired() {
    let (_server, client) = serving("/api/v1/key", json(401, INVALID_KEY)).await;
    assert_eq!(
        client.key("sk-or-v1-test", now()).await,
        Err(ProviderError::SignInExpired)
    );
}

#[tokio::test]
async fn rate_limits_carry_retry_after() {
    let limited = json(429, RATE_LIMITED).insert_header("retry-after", "120");
    let (_server, client) = serving("/api/v1/key", limited).await;
    assert_eq!(
        client.key("sk-or-v1-test", now()).await,
        Err(ProviderError::rate_limited(Some(
            SignedDuration::from_secs(120)
        )))
    );
}

#[tokio::test]
async fn server_errors_are_network_errors_and_garbage_is_invalid() {
    let (_server, client) = serving("/api/v1/key", json(503, "")).await;
    assert!(matches!(
        client.key("sk-or-v1-test", now()).await,
        Err(ProviderError::Network(_))
    ));
    let (_server, client) = serving("/api/v1/key", json(200, r#"{"data":{"usage":"x"}}"#)).await;
    assert!(matches!(
        client.key("sk-or-v1-test", now()).await,
        Err(ProviderError::InvalidResponse(_))
    ));
}

#[tokio::test]
async fn credits_parse_for_a_management_key() {
    let (_server, client) = serving("/api/v1/credits", json(200, CREDITS)).await;
    assert_eq!(
        client.credits("sk-or-v1-test", now()).await,
        Credits::Available(RawCredits {
            total_credits: Usd(MicroUsd(150_000_000)),
            total_usage: Usd(MicroUsd(41_876_421)),
        })
    );
}

#[tokio::test]
async fn credits_refused_to_a_regular_key_need_a_management_key() {
    for status in [401, 403] {
        let (_server, client) = serving("/api/v1/credits", json(status, CREDITS_FORBIDDEN)).await;
        assert_eq!(
            client.credits("sk-or-v1-test", now()).await,
            Credits::NeedsManagementKey
        );
    }
}

#[tokio::test]
async fn other_credit_failures_are_reported_as_unavailable() {
    let (_server, client) = serving("/api/v1/credits", json(500, "")).await;
    assert!(matches!(
        client.credits("sk-or-v1-test", now()).await,
        Credits::Unavailable(ProviderError::Network(_))
    ));
}
