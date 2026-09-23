use headroom_core::pace::Tone;
use headroom_core::quota::{LimitsSource, Notice, WindowId};
use jiff::SignedDuration;
use wiremock::matchers::method;
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::offline::{OFFLINE_NOTICE, SIGN_IN_EXPIRED_NOTICE};
use super::test_support::{
    ACCOUNT_ID, Setup, USER_ID, access_token, at, auth_document_with, id_token_for, server_with,
    set_mtime, write_auth, write_rollout,
};
use super::*;

const RATE_LIMITS: &str = include_str!("fixtures/rollout_rate_limits.jsonl");
const EXPIRED: &str = "2026-09-23T09:00:00Z";
const FRESH: &str = "2026-09-24T00:00:00Z";
const AFTER_LOGS: &str = "2026-09-23T00:00:00Z";
const BEFORE_LOGS: &str = "2026-09-01T00:00:00Z";

async fn unreachable_server() -> MockServer {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&server)
        .await;
    server
}

fn sign_in_as(setup: &Setup, user_id: &str, account_id: &str, expires_at: &str) {
    let id_token = id_token_for(user_id, account_id, Some(at(BEFORE_LOGS)));
    let token = access_token(at(expires_at));
    write_auth(
        &setup.cli_home(),
        &auth_document_with(&id_token, account_id, &token),
    );
}

fn sign_in_without_auth_time(setup: &Setup, signed_in_at: &str) {
    let id_token = id_token_for(USER_ID, ACCOUNT_ID, None);
    let token = access_token(at(EXPIRED));
    write_auth(
        &setup.cli_home(),
        &auth_document_with(&id_token, ACCOUNT_ID, &token),
    );
    set_mtime(&setup.cli_home().join("auth.json"), signed_in_at);
}

fn account_changed(setup: &Setup) -> ProviderError {
    ProviderError::LocalData(format!(
        "the Codex account signed in at {} has changed",
        setup.cli_home().display()
    ))
}

fn warning(text: &str) -> Vec<Notice> {
    vec![Notice {
        tone: Tone::Warning,
        text: text.to_owned(),
    }]
}

async fn discovered_then_switched(setup: &Setup, expires_at: &str) -> AccountRef {
    sign_in_as(setup, USER_ID, ACCOUNT_ID, expires_at);
    let accounts = setup.provider(DEFAULT_API_BASE).discover().await.unwrap();
    sign_in_as(setup, "user-fake0002", "acct-fake0002", expires_at);
    accounts[0].clone()
}

#[tokio::test]
async fn switched_account_is_reported_instead_of_live_limits() {
    let setup = Setup::new();
    let account = discovered_then_switched(&setup, FRESH).await;
    let server = unreachable_server().await;
    let result = setup.provider(&server.uri()).fetch_limits(&account).await;
    assert_eq!(result, Err(account_changed(&setup)));
}

#[tokio::test]
async fn switched_account_is_reported_instead_of_offline_limits() {
    let setup = Setup::new();
    write_rollout(&setup.cli_home(), "sessions/rollout.jsonl", RATE_LIMITS);
    let account = discovered_then_switched(&setup, EXPIRED).await;
    let server = unreachable_server().await;
    let result = setup.provider(&server.uri()).fetch_limits(&account).await;
    assert_eq!(result, Err(account_changed(&setup)));
}

#[tokio::test]
async fn expired_sign_in_falls_back_to_logs_with_a_warning() {
    let setup = Setup::new();
    setup.sign_in(EXPIRED);
    write_rollout(&setup.cli_home(), "sessions/rollout.jsonl", RATE_LIMITS);
    let server = unreachable_server().await;
    let snapshot = setup
        .provider(&server.uri())
        .fetch_limits(&setup.account())
        .await
        .unwrap();
    assert!(matches!(snapshot.source, LimitsSource::LocalLog { .. }));
    assert_eq!(snapshot.windows[0].id, WindowId::Weekly);
    assert_eq!(snapshot.notices, warning(SIGN_IN_EXPIRED_NOTICE));
}

#[tokio::test]
async fn network_and_rejected_sign_in_fall_back_to_logs_with_a_warning() {
    let setup = Setup::new();
    setup.sign_in(FRESH);
    write_rollout(&setup.cli_home(), "sessions/rollout.jsonl", RATE_LIMITS);
    for (status, notice) in [(500, OFFLINE_NOTICE), (401, SIGN_IN_EXPIRED_NOTICE)] {
        let server = server_with(ResponseTemplate::new(status)).await;
        let snapshot = setup
            .provider(&server.uri())
            .fetch_limits(&setup.account())
            .await
            .unwrap();
        assert!(matches!(snapshot.source, LimitsSource::LocalLog { .. }));
        assert_eq!(snapshot.notices, warning(notice));
    }
}

#[tokio::test]
async fn rate_limit_is_returned_even_when_logs_exist() {
    let setup = Setup::new();
    setup.sign_in(FRESH);
    write_rollout(&setup.cli_home(), "sessions/rollout.jsonl", RATE_LIMITS);
    let server = server_with(ResponseTemplate::new(429).insert_header("retry-after", "3600")).await;
    let result = setup
        .provider(&server.uri())
        .fetch_limits(&setup.account())
        .await;
    assert_eq!(
        result,
        Err(ProviderError::RateLimited {
            retry_after: Some(SignedDuration::from_secs(3600))
        })
    );
}

#[tokio::test]
async fn without_logs_the_original_error_is_returned() {
    let setup = Setup::new();
    setup.sign_in(FRESH);
    let server = server_with(ResponseTemplate::new(500)).await;
    let result = setup
        .provider(&server.uri())
        .fetch_limits(&setup.account())
        .await;
    assert!(matches!(result, Err(ProviderError::Network(_))));
}

#[tokio::test]
async fn logs_from_before_the_login_time_claim_are_ignored() {
    let setup = Setup::new();
    write_rollout(&setup.cli_home(), "sessions/rollout.jsonl", RATE_LIMITS);
    let id_token = id_token_for(USER_ID, ACCOUNT_ID, Some(at(AFTER_LOGS)));
    let token = access_token(at(EXPIRED));
    write_auth(
        &setup.cli_home(),
        &auth_document_with(&id_token, ACCOUNT_ID, &token),
    );
    let result = setup
        .provider(DEFAULT_API_BASE)
        .fetch_limits(&setup.account())
        .await;
    assert_eq!(result, Err(ProviderError::SignInExpired));
}

#[tokio::test]
async fn without_a_login_time_claim_the_auth_file_time_decides() {
    let setup = Setup::new();
    write_rollout(&setup.cli_home(), "sessions/rollout.jsonl", RATE_LIMITS);
    let provider = setup.provider(DEFAULT_API_BASE);
    sign_in_without_auth_time(&setup, AFTER_LOGS);
    let result = provider.fetch_limits(&setup.account()).await;
    assert_eq!(result, Err(ProviderError::SignInExpired));
    sign_in_without_auth_time(&setup, BEFORE_LOGS);
    let snapshot = provider.fetch_limits(&setup.account()).await.unwrap();
    assert!(matches!(snapshot.source, LimitsSource::LocalLog { .. }));
}

#[tokio::test]
async fn invalid_response_does_not_fall_back() {
    let setup = Setup::new();
    setup.sign_in(FRESH);
    write_rollout(&setup.cli_home(), "sessions/rollout.jsonl", RATE_LIMITS);
    let server = server_with(ResponseTemplate::new(200).set_body_string("nope")).await;
    let result = setup
        .provider(&server.uri())
        .fetch_limits(&setup.account())
        .await;
    assert!(matches!(result, Err(ProviderError::InvalidResponse(_))));
}

#[tokio::test]
async fn signed_out_account_gets_no_offline_data() {
    let setup = Setup::new();
    write_rollout(&setup.cli_home(), "sessions/rollout.jsonl", RATE_LIMITS);
    let result = setup
        .provider(DEFAULT_API_BASE)
        .fetch_limits(&setup.account())
        .await;
    assert_eq!(result, Err(ProviderError::NotSignedIn));
}
