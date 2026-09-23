use jiff::SignedDuration;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::*;

const QUOTA: &str = include_str!("fixtures/quota_tokens.json");
const INVALID_KEY: &str = include_str!("fixtures/invalid_key.json");
const RATE_LIMITED: &str = include_str!("fixtures/rate_limited.json");
const SUBSCRIPTION: &str = include_str!("fixtures/subscription.json");

fn now() -> Timestamp {
    "2026-09-23T10:00:00Z".parse().unwrap()
}

fn client(server: &MockServer) -> QuotaClient {
    QuotaClient::new(crate::http::client().unwrap(), &server.uri())
}

async fn answer(server: &MockServer, route: &str, auth: &str, reply: ResponseTemplate, calls: u64) {
    Mock::given(method("GET"))
        .and(path(route))
        .and(header("authorization", auth))
        .and(header("accept", "application/json"))
        .respond_with(reply)
        .expect(calls)
        .mount(server)
        .await;
}

fn json(status: u16, body: &str) -> ResponseTemplate {
    ResponseTemplate::new(status).set_body_string(body)
}

#[tokio::test]
async fn bearer_is_tried_first() {
    let server = MockServer::start().await;
    answer(&server, QUOTA_PATH, "Bearer zk-test", json(200, QUOTA), 1).await;
    answer(&server, QUOTA_PATH, "zk-test", json(200, QUOTA), 0).await;
    let (body, scheme) = client(&server).quota("zk-test", now()).await.unwrap();
    assert_eq!(body, QUOTA.as_bytes());
    assert_eq!(scheme, Scheme::Bearer);
}

#[tokio::test]
async fn a_rejected_bearer_is_retried_once_with_the_raw_key() {
    let server = MockServer::start().await;
    answer(
        &server,
        QUOTA_PATH,
        "Bearer zk-test",
        json(401, INVALID_KEY),
        1,
    )
    .await;
    answer(&server, QUOTA_PATH, "zk-test", json(200, QUOTA), 1).await;
    answer(
        &server,
        SUBSCRIPTION_PATH,
        "zk-test",
        json(200, SUBSCRIPTION),
        1,
    )
    .await;
    let client = client(&server);
    let (_, scheme) = client.quota("zk-test", now()).await.unwrap();
    assert_eq!(scheme, Scheme::Raw);
    let body = client
        .subscriptions("zk-test", scheme, now())
        .await
        .unwrap();
    assert_eq!(body, SUBSCRIPTION.as_bytes());
}

#[tokio::test]
async fn a_key_rejected_both_ways_is_signed_out() {
    let server = MockServer::start().await;
    answer(
        &server,
        QUOTA_PATH,
        "Bearer zk-test",
        json(401, INVALID_KEY),
        1,
    )
    .await;
    answer(&server, QUOTA_PATH, "zk-test", json(401, INVALID_KEY), 1).await;
    assert_eq!(
        client(&server).quota("zk-test", now()).await,
        Err(ProviderError::SignInExpired)
    );
}

#[tokio::test]
async fn forbidden_means_signed_out_unless_it_names_the_plan() {
    let server = MockServer::start().await;
    answer(
        &server,
        QUOTA_PATH,
        "Bearer zk-a",
        json(403, INVALID_KEY),
        1,
    )
    .await;
    let plan_body = r#"{"success":false,"msg":"No active subscription plan"}"#;
    answer(&server, QUOTA_PATH, "Bearer zk-b", json(403, plan_body), 1).await;
    let client = client(&server);
    assert_eq!(
        client.quota("zk-a", now()).await,
        Err(ProviderError::SignInExpired)
    );
    assert!(matches!(
        client.quota("zk-b", now()).await,
        Err(ProviderError::NoSubscription { .. })
    ));
}

#[tokio::test]
async fn rate_limits_carry_retry_after_and_are_not_retried() {
    let server = MockServer::start().await;
    let limited = json(429, RATE_LIMITED).insert_header("retry-after", "45");
    answer(&server, QUOTA_PATH, "Bearer zk-test", limited, 1).await;
    answer(&server, QUOTA_PATH, "zk-test", json(200, QUOTA), 0).await;
    assert_eq!(
        client(&server).quota("zk-test", now()).await,
        Err(ProviderError::RateLimited {
            retry_after: Some(SignedDuration::from_secs(45))
        })
    );
}

#[tokio::test]
async fn server_errors_are_network_errors() {
    let server = MockServer::start().await;
    answer(&server, QUOTA_PATH, "Bearer zk-test", json(502, ""), 1).await;
    assert_eq!(
        client(&server).quota("zk-test", now()).await,
        Err(ProviderError::Network("Z.ai returned HTTP 502".into()))
    );
}

#[test]
fn keys_that_cannot_be_sent_are_refused_before_the_network() {
    assert!(authorization("bad\nkey", Scheme::Raw).is_err());
    let value = authorization("zk-test", Scheme::Bearer).unwrap();
    assert!(value.is_sensitive());
    assert_eq!(value, "Bearer zk-test");
}
