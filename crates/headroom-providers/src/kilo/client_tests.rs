use headroom_core::units::MicroUsd;
use wiremock::matchers::{header, header_exists, method, path};
use wiremock::{Mock, MockServer, Request, ResponseTemplate};

use super::*;

const BALANCE: &str = include_str!("fixtures/balance.json");
const DEPLETED: &str = include_str!("fixtures/balance_depleted.json");
const PROFILE: &str = include_str!("fixtures/profile.json");
const TOKEN: &str = "kilo-test-token";

fn now() -> Timestamp {
    "2026-09-24T10:00:00Z".parse().unwrap()
}

async fn serving(endpoint: &str, response: ResponseTemplate) -> (MockServer, KiloClient) {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(endpoint))
        .and(header("authorization", format!("Bearer {TOKEN}")))
        .and(header("accept", "application/json"))
        .respond_with(response)
        .mount(&server)
        .await;
    let client = KiloClient::new(
        crate::http::client().unwrap(),
        &format!("{}/", server.uri()),
    );
    (server, client)
}

fn json(status: u16, body: &str) -> ResponseTemplate {
    ResponseTemplate::new(status).set_body_string(body)
}

#[tokio::test]
async fn the_personal_balance_parses_exactly_without_an_organization_header() {
    let (server, client) = serving("/api/profile/balance", json(200, BALANCE)).await;
    let balance = client.balance(TOKEN, None, now()).await.unwrap();
    assert_eq!(balance.balance, Usd(MicroUsd(18_123_456)));
    assert_eq!(balance.is_depleted, Some(false));
    let requests: Vec<Request> = server.received_requests().await.unwrap();
    assert!(!requests[0].headers.contains_key(ORGANIZATION_HEADER));
}

#[tokio::test]
async fn an_organization_balance_sends_the_organization_header() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/profile/balance"))
        .and(header(ORGANIZATION_HEADER, "org-0000-fake"))
        .and(header_exists("authorization"))
        .respond_with(json(200, DEPLETED))
        .mount(&server)
        .await;
    let client = KiloClient::new(crate::http::client().unwrap(), &server.uri());
    let balance = client
        .balance(TOKEN, Some("org-0000-fake"), now())
        .await
        .unwrap();
    assert_eq!(balance.balance, Usd(MicroUsd(-421_300)));
    assert_eq!(balance.is_depleted, Some(true));
}

#[tokio::test]
async fn the_profile_gives_the_email() {
    let (_server, client) = serving("/api/profile", json(200, PROFILE)).await;
    let profile = client.profile(TOKEN, now()).await.unwrap();
    assert_eq!(profile.user.email.as_deref(), Some("user@example.com"));
}

#[tokio::test]
async fn http_failures_map_to_provider_errors() {
    for status in [401, 403] {
        let (_server, client) = serving("/api/profile/balance", json(status, "")).await;
        assert_eq!(
            client.balance(TOKEN, None, now()).await,
            Err(ProviderError::SignInExpired)
        );
    }
    let limited = json(429, "").insert_header("retry-after", "90");
    let (_server, client) = serving("/api/profile/balance", limited).await;
    assert_eq!(
        client.balance(TOKEN, None, now()).await,
        Err(ProviderError::rate_limited(Some(
            SignedDuration::from_secs(90)
        )))
    );
    let (_server, client) = serving("/api/profile/balance", json(500, "")).await;
    assert!(matches!(
        client.balance(TOKEN, None, now()).await,
        Err(ProviderError::Network(_))
    ));
}

#[tokio::test]
async fn balances_that_are_not_exact_micro_usd_are_invalid() {
    for body in [
        r#"{"balance":0.0000001,"isDepleted":false}"#,
        r#"{"balance":"4.5"}"#,
        r#"{"isDepleted":true}"#,
    ] {
        let (_server, client) = serving("/api/profile/balance", json(200, body)).await;
        assert!(
            matches!(
                client.balance(TOKEN, None, now()).await,
                Err(ProviderError::InvalidResponse(_))
            ),
            "{body}"
        );
    }
}
