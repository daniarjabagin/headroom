use std::fs;

use headroom_core::account::{AccountId, CredentialOwner};
use headroom_core::quota::{BalanceAmount, WindowId};
use headroom_core::units::MicroUsd;
use tempfile::TempDir;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::*;

const FULL: &str = include_str!("fixtures/user_status.json");
const NO_PLAN: &str = include_str!("fixtures/user_status_no_plan.json");
const CREDENTIALS: &str = include_str!("fixtures/credentials.toml");

fn fixed_now() -> Timestamp {
    "2026-09-23T10:00:00Z".parse().unwrap()
}

async fn server_with(status: u16, body: &str) -> MockServer {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(
            "/exa.seat_management_pb.SeatManagementService/GetUserStatus",
        ))
        .respond_with(ResponseTemplate::new(status).set_body_string(body))
        .mount(&server)
        .await;
    server
}

fn signed_in_provider(root: &TempDir, server: &MockServer) -> DevinProvider {
    let mut config = DevinConfig::for_home(root.path());
    config.api_base = server.uri();
    fs::create_dir_all(config.cli_dir()).unwrap();
    fs::write(config.cli_dir().join(auth::CREDENTIALS_FILE), CREDENTIALS).unwrap();
    DevinProvider::with_clock(config, crate::http::client().unwrap(), fixed_now)
}

async fn only_account(provider: &DevinProvider) -> AccountRef {
    let accounts = provider.discover().await.unwrap();
    assert_eq!(accounts.len(), 1);
    accounts.into_iter().next().unwrap()
}

#[tokio::test]
async fn a_cli_sign_in_fetches_live_limits() {
    let root = tempfile::tempdir().unwrap();
    let server = server_with(200, FULL).await;
    let provider = signed_in_provider(&root, &server);
    let account = only_account(&provider).await;
    assert_eq!(account.owner, CredentialOwner::Cli);
    let snapshot = provider.fetch_limits(&account).await.unwrap();
    assert_eq!(
        snapshot.identity.email.as_deref(),
        Some("user@example.test")
    );
    assert_eq!(snapshot.identity.plan.as_deref(), Some("Max"));
    assert_eq!(snapshot.identity.account_id(&ID), account.id);
    assert_eq!(snapshot.fetched_at, fixed_now());
    assert_eq!(snapshot.source, LimitsSource::Live);
    let ids: Vec<_> = snapshot.windows.iter().map(|w| w.id.clone()).collect();
    assert_eq!(ids, [WindowId::Other("daily".into()), WindowId::Weekly]);
    assert_eq!(
        snapshot.balances[0].amount,
        BalanceAmount::Usd(MicroUsd(964_220_000))
    );
}

#[tokio::test]
async fn failures_map_to_user_facing_errors() {
    let cases = [
        (200, NO_PLAN, "no_subscription"),
        (401, "{}", "sign_in_expired"),
        (429, "{}", "rate_limited"),
    ];
    for (status, body, kind) in cases {
        let root = tempfile::tempdir().unwrap();
        let server = server_with(status, body).await;
        let provider = signed_in_provider(&root, &server);
        let account = only_account(&provider).await;
        let error = provider.fetch_limits(&account).await.unwrap_err();
        let json = serde_json::to_value(&error).unwrap();
        assert_eq!(json["kind"], kind, "{status}");
    }
}

#[tokio::test]
async fn a_signed_out_or_replaced_account_is_reported_without_a_request() {
    let root = tempfile::tempdir().unwrap();
    let server = server_with(200, FULL).await;
    let provider = signed_in_provider(&root, &server);
    let account = only_account(&provider).await;
    let replaced = AccountRef {
        id: AccountId("devin:000000000000".into()),
        ..account.clone()
    };
    assert!(matches!(
        provider.fetch_limits(&replaced).await,
        Err(ProviderError::AccountChanged(_))
    ));
    fs::remove_file(account.home.join(auth::CREDENTIALS_FILE)).unwrap();
    assert_eq!(
        provider.fetch_limits(&account).await.unwrap_err(),
        ProviderError::NotSignedIn
    );
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn a_login_home_is_found_and_devin_has_no_local_usage() {
    let root = tempfile::tempdir().unwrap();
    let server = server_with(200, FULL).await;
    let provider = signed_in_provider(&root, &server);
    let home = root.path().join("login");
    let login = match DESCRIPTOR.default_method() {
        Some(AddAccountMethod::CliLogin(login)) => login,
        other => panic!("{other:?}"),
    };
    fs::create_dir_all(login.credentials_path(&home).parent().unwrap()).unwrap();
    fs::write(login.credentials_path(&home), CREDENTIALS).unwrap();
    let account = provider.account_at(&home).await.unwrap().unwrap();
    assert_eq!(account.owner, CredentialOwner::Headroom);
    assert!(provider.fetch_limits(&account).await.is_ok());
    assert!(provider.usage_homes().await.unwrap().is_empty());
    let mut cursors = LogCursors::default();
    assert!(provider.read_usage(&home, &mut cursors).unwrap().is_empty());
}
