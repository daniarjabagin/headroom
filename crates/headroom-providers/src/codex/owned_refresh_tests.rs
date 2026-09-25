use std::fs;

use headroom_core::quota::LimitsSource;
use serde_json::{Value, json};
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::test_support::{Setup, access_token, at, auth_document, id_token, write_auth};
use super::*;

const FULL: &str = include_str!("fixtures/usage_full.json");
const EXPIRED: &str = "2026-09-23T09:00:00Z";

async fn server(fresh_access: &str, token_calls: u64) -> MockServer {
    let server = MockServer::start().await;
    let body =
        json!({ "access_token": fresh_access, "id_token": id_token(), "refresh_token": "rt-new" });
    Mock::given(method("POST"))
        .and(path("/oauth/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .expect(token_calls)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/backend-api/wham/usage"))
        .and(header("authorization", format!("Bearer {fresh_access}")))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(FULL.as_bytes().to_vec(), "application/json"),
        )
        .mount(&server)
        .await;
    server
}

fn owned_account(setup: &Setup) -> AccountRef {
    AccountRef {
        home: setup.headroom_home("owned"),
        owner: CredentialOwner::Headroom,
        ..setup.account()
    }
}

#[tokio::test]
async fn an_expired_headroom_sign_in_is_refreshed_and_saved() {
    let setup = Setup::new();
    let account = owned_account(&setup);
    write_auth(&account.home, &auth_document(&access_token(at(EXPIRED))));
    let fresh_access = access_token(at("2026-09-23T20:00:00Z"));
    let server = server(&fresh_access, 1).await;
    let snapshot = setup
        .provider(&server.uri())
        .fetch_limits(&account)
        .await
        .unwrap();
    assert_eq!(snapshot.source, LimitsSource::Live);
    let stored: Value =
        serde_json::from_slice(&fs::read(account.home.join("auth.json")).unwrap()).unwrap();
    assert_eq!(stored["tokens"]["access_token"], fresh_access.as_str());
    assert_eq!(stored["tokens"]["refresh_token"], "rt-new");
}

#[tokio::test]
async fn an_expired_cli_sign_in_is_never_refreshed_or_rewritten() {
    let setup = Setup::new();
    setup.sign_in(EXPIRED);
    let file = setup.cli_home().join("auth.json");
    let before = fs::read(&file).unwrap();
    let server = server(&access_token(at("2026-09-23T20:00:00Z")), 0).await;
    let result = setup
        .provider(&server.uri())
        .fetch_limits(&setup.account())
        .await;
    assert_eq!(result, Err(ProviderError::SignInExpired));
    assert_eq!(fs::read(&file).unwrap(), before);
}

#[tokio::test]
async fn a_refresh_that_signs_in_someone_else_is_an_account_change() {
    let setup = Setup::new();
    let account = owned_account(&setup);
    write_auth(&account.home, &auth_document(&access_token(at(EXPIRED))));
    let server = MockServer::start().await;
    let other = super::test_support::id_token_for("user-other", "acct-other", None);
    let body = json!({ "access_token": "new", "id_token": other });
    Mock::given(method("POST"))
        .and(path("/oauth/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(&server)
        .await;
    let result = setup.provider(&server.uri()).fetch_limits(&account).await;
    assert!(matches!(result, Err(ProviderError::AccountChanged(_))));
}
