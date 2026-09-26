use jiff::SignedDuration;
use wiremock::matchers::{body_string, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::*;

fn now() -> Timestamp {
    "2026-09-23T10:00:00Z".parse().unwrap()
}

async fn serving(status: u16, body: &str) -> (MockServer, KimiClient) {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/coding/v1/usages"))
        .and(header("authorization", "Bearer sk-kimi-test"))
        .respond_with(ResponseTemplate::new(status).set_body_string(body))
        .mount(&server)
        .await;
    let client = client_for(&server);
    (server, client)
}

fn client_for(server: &MockServer) -> KimiClient {
    let base = format!("{}/coding/v1/", server.uri());
    KimiClient::new(Client::new(), &base, &server.uri())
}

async fn usages_error_for(status: u16, body: &str) -> ProviderError {
    let (_server, client) = serving(status, body).await;
    client.usages("sk-kimi-test", now()).await.unwrap_err()
}

#[tokio::test]
async fn usages_are_fetched_with_the_bearer_key() {
    let (_server, client) = serving(200, include_str!("fixtures/usages_counts.json")).await;
    let raw = client.usages("sk-kimi-test", now()).await.unwrap();
    assert_eq!(raw.limits.map(|limits| limits.len()), Some(1));
}

#[tokio::test]
async fn statuses_map_to_typed_errors() {
    let invalid = include_str!("fixtures/invalid_key.json");
    assert_eq!(
        usages_error_for(401, invalid).await,
        ProviderError::SignInExpired
    );
    for status in [402, 403, 404] {
        assert_eq!(
            usages_error_for(status, "{}").await,
            ProviderError::NoSubscription {
                detail: NO_PLAN.into()
            },
            "{status}"
        );
    }
    assert!(matches!(
        usages_error_for(503, "").await,
        ProviderError::Network(_)
    ));
    assert!(matches!(
        usages_error_for(418, "").await,
        ProviderError::InvalidResponse(_)
    ));
    assert!(matches!(
        usages_error_for(200, "<html>").await,
        ProviderError::InvalidResponse(_)
    ));
}

#[tokio::test]
async fn rate_limits_carry_retry_after() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(429).insert_header("retry-after", "120"))
        .mount(&server)
        .await;
    let error = client_for(&server).usages("k", now()).await.unwrap_err();
    assert_eq!(
        error,
        ProviderError::rate_limited(Some(SignedDuration::from_secs(120)))
    );
}

#[tokio::test]
async fn refresh_posts_the_form_and_reads_new_tokens() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/oauth/token"))
        .and(header("content-type", FORM_TYPE))
        .and(body_string(format!(
            "client_id={CLIENT_ID}&grant_type=refresh_token&refresh_token=r%2Fx%3Dy"
        )))
        .respond_with(
            ResponseTemplate::new(200).set_body_string(include_str!("fixtures/refreshed.json")),
        )
        .expect(1)
        .mount(&server)
        .await;
    let refreshed = client_for(&server).refresh("r/x=y", now()).await;
    assert_eq!(
        refreshed.ok().unwrap().access().expose(),
        "fake-access-token-2"
    );
}

#[tokio::test]
async fn a_rejected_refresh_means_signing_in_again() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(400).set_body_string(r#"{"error":"invalid_grant"}"#))
        .mount(&server)
        .await;
    let error = client_for(&server).refresh("r", now()).await.err().unwrap();
    assert_eq!(error, ProviderError::SignInExpired);
}
