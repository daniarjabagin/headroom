use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::*;
use crate::decimal::ExactMicros;

const BALANCE: &str = include_str!("fixtures/balance.json");
const INVALID_KEY: &str = include_str!("fixtures/invalid_key.json");
const KEY: &str = "sk-moonshot-test";

fn now() -> Timestamp {
    "2026-09-24T10:00:00Z".parse().unwrap()
}

async fn host(status: u16, body: &str) -> MockServer {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/users/me/balance"))
        .and(header("authorization", format!("Bearer {KEY}")))
        .and(header("accept", "application/json"))
        .respond_with(ResponseTemplate::new(status).set_body_string(body))
        .mount(&server)
        .await;
    server
}

fn client(global: &MockServer, mainland: &MockServer) -> BalanceClient {
    BalanceClient::new(
        crate::http::client().unwrap(),
        &format!("{}/", global.uri()),
        &mainland.uri(),
    )
}

#[tokio::test]
async fn the_documented_numbers_are_read_exactly() {
    let global = host(200, BALANCE).await;
    let mainland = host(401, INVALID_KEY).await;
    let balance = client(&global, &mainland)
        .balance(Region::Global, KEY, now())
        .await
        .unwrap();
    assert_eq!(
        balance,
        RawBalance {
            available: ExactMicros(49_588_940),
            voucher: ExactMicros(46_588_930),
            cash: ExactMicros(3_000_010),
        }
    );
}

#[tokio::test]
async fn a_key_unknown_globally_is_tried_on_the_mainland_host() {
    let global = host(401, INVALID_KEY).await;
    let mainland = host(200, BALANCE).await;
    let (region, _) = client(&global, &mainland)
        .locate(Region::Global, KEY, now())
        .await
        .unwrap();
    assert_eq!(region, Region::Mainland);
    let (region, _) = client(&mainland, &global)
        .locate(Region::Mainland, KEY, now())
        .await
        .unwrap();
    assert_eq!(region, Region::Global);
}

#[tokio::test]
async fn a_key_unknown_on_both_hosts_is_signed_out() {
    let global = host(401, INVALID_KEY).await;
    let mainland = host(403, INVALID_KEY).await;
    assert_eq!(
        client(&global, &mainland)
            .locate(Region::Global, KEY, now())
            .await,
        Err(ProviderError::SignInExpired)
    );
}

#[tokio::test]
async fn other_failures_do_not_switch_hosts() {
    let global = host(503, "").await;
    let mainland = host(200, BALANCE).await;
    assert_eq!(
        client(&global, &mainland)
            .locate(Region::Global, KEY, now())
            .await,
        Err(ProviderError::Network("Moonshot returned HTTP 503".into()))
    );
}

#[tokio::test]
async fn a_failed_envelope_or_inexact_number_is_invalid() {
    let failed = r#"{"code":5,"status":false,"data":null,"scode":"0x5"}"#;
    let global = host(200, failed).await;
    assert_eq!(
        client(&global, &global)
            .balance(Region::Global, KEY, now())
            .await,
        Err(ProviderError::InvalidResponse(
            "Moonshot balance answered code 5, status false, without a balance".into()
        ))
    );
    let scientific = r#"{"code":0,"status":true,"data":{"available_balance":4.9e1,"voucher_balance":0,"cash_balance":0}}"#;
    let global = host(200, scientific).await;
    assert!(matches!(
        client(&global, &global)
            .balance(Region::Global, KEY, now())
            .await,
        Err(ProviderError::InvalidResponse(_))
    ));
}
