use jiff::SignedDuration;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::*;

const BALANCE: &str = include_str!("fixtures/current_balance.json");
const INVALID_KEY: &str = include_str!("fixtures/invalid_key.json");
const KEY: &str = "poe-test-key";

fn now() -> Timestamp {
    "2026-09-24T10:00:00Z".parse().unwrap()
}

async fn serving(response: ResponseTemplate) -> (MockServer, PoeClient) {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/usage/current_balance"))
        .and(header("authorization", format!("Bearer {KEY}")))
        .and(header("accept", "application/json"))
        .respond_with(response)
        .mount(&server)
        .await;
    let client = PoeClient::new(
        crate::http::client().unwrap(),
        &format!("{}/", server.uri()),
    );
    (server, client)
}

fn json(status: u16, body: &str) -> ResponseTemplate {
    ResponseTemplate::new(status).set_body_string(body)
}

#[tokio::test]
async fn the_point_balance_parses_with_bearer_auth() {
    let (_server, client) = serving(json(200, BALANCE)).await;
    assert_eq!(
        client.balance(KEY, now()).await,
        Ok(RawBalance {
            current_point_balance: 842_150
        })
    );
}

#[tokio::test]
async fn an_invalid_key_means_sign_in_expired() {
    let (_server, client) = serving(json(401, INVALID_KEY)).await;
    assert_eq!(
        client.balance(KEY, now()).await,
        Err(ProviderError::SignInExpired)
    );
}

#[tokio::test]
async fn rate_limits_carry_retry_after() {
    let (_server, client) = serving(json(429, "").insert_header("retry-after", "30")).await;
    assert_eq!(
        client.balance(KEY, now()).await,
        Err(ProviderError::RateLimited {
            retry_after: Some(SignedDuration::from_secs(30))
        })
    );
}

#[tokio::test]
async fn server_errors_are_network_errors_and_bad_bodies_are_invalid() {
    let (_server, client) = serving(json(502, "")).await;
    assert!(matches!(
        client.balance(KEY, now()).await,
        Err(ProviderError::Network(_))
    ));
    for body in [
        r#"{"current_point_balance":-5}"#,
        r#"{"current_point_balance":1.5}"#,
        "{}",
    ] {
        let (_server, client) = serving(json(200, body)).await;
        assert!(
            matches!(
                client.balance(KEY, now()).await,
                Err(ProviderError::InvalidResponse(_))
            ),
            "{body}"
        );
    }
}
