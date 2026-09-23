use headroom_core::account::CredentialOwner;
use headroom_core::quota::WindowId;
use tempfile::TempDir;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::test_support::{fake_gh_printing_stored_tokens, sign_in};
use super::*;

const PRO: &str = include_str!("fixtures/user_pro.json");
const NO_PLAN: &str = include_str!("fixtures/user_no_plan.json");
const MULTI: &str = include_str!("fixtures/hosts_multi.yml");

fn fixed_now() -> Timestamp {
    "2026-09-23T10:00:00Z".parse().unwrap()
}

async fn server_with(status: u16, body: &str, token: &str) -> MockServer {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/copilot_internal/user"))
        .and(header("authorization", format!("token {token}").as_str()))
        .respond_with(ResponseTemplate::new(status).set_body_string(body))
        .mount(&server)
        .await;
    server
}

fn provider(root: &TempDir, server: &MockServer) -> CopilotProvider {
    let mut config = CopilotConfig::for_home(root.path());
    config.api_base = server.uri();
    config.gh_program = fake_gh_printing_stored_tokens(root.path());
    sign_in(
        &config.gh_config_dir,
        MULTI,
        &[("octocat", "gho_fake_cat"), ("octo-work", "gho_fake_work")],
    );
    CopilotProvider::with_clock(config, crate::http::client().unwrap(), fixed_now)
}

async fn account_for(provider: &CopilotProvider, login: &str) -> AccountRef {
    let id = headroom_core::account::AccountId::from_stable_key(&ID, &accounts::stable_key(login));
    let accounts = provider.discover().await.unwrap();
    accounts.into_iter().find(|a| a.id == id).unwrap()
}

#[tokio::test]
async fn each_gh_account_fetches_with_its_own_token() {
    let root = tempfile::tempdir().unwrap();
    let server = server_with(200, PRO, "gho_fake_cat").await;
    let provider = provider(&root, &server);
    assert_eq!(provider.discover().await.unwrap().len(), 2);
    let account = account_for(&provider, "octocat").await;
    assert_eq!(account.owner, CredentialOwner::Cli);
    let snapshot = provider.fetch_limits(&account).await.unwrap();
    assert_eq!(snapshot.identity.email.as_deref(), Some("octocat"));
    assert_eq!(snapshot.identity.plan.as_deref(), Some("Individual Pro"));
    assert_eq!(snapshot.identity.account_id(&ID), account.id);
    assert_eq!(snapshot.windows[0].id, WindowId::Other("credits".into()));
    assert_eq!(snapshot.fetched_at, fixed_now());
    assert_eq!(snapshot.source, LimitsSource::Live);
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
        let server = server_with(status, body, "gho_fake_work").await;
        let provider = provider(&root, &server);
        let account = account_for(&provider, "octo-work").await;
        let error = provider.fetch_limits(&account).await.unwrap_err();
        let json = serde_json::to_value(&error).unwrap();
        assert_eq!(json["kind"], kind, "{status}");
    }
}

#[tokio::test]
async fn a_user_without_a_stored_token_must_sign_in_again_without_a_request() {
    let root = tempfile::tempdir().unwrap();
    let server = server_with(200, PRO, "gho_fake_cat").await;
    let provider = provider(&root, &server);
    let account = account_for(&provider, "octocat").await;
    std::fs::remove_file(account.home.join("fake-tokens/octocat")).unwrap();
    assert_eq!(
        provider.fetch_limits(&account).await.unwrap_err(),
        ProviderError::SignInExpired
    );
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn a_login_home_is_found_and_copilot_has_no_local_usage() {
    let root = tempfile::tempdir().unwrap();
    let server = server_with(200, PRO, "gho_fake_mona").await;
    let provider = provider(&root, &server);
    let home = root.path().join("login");
    let hosts = "github.com:\n    users:\n        mona:\n    user: mona\n";
    sign_in(&home, hosts, &[("mona", "gho_fake_mona")]);
    let login = match DESCRIPTOR.default_method() {
        Some(AddAccountMethod::CliLogin(login)) => login,
        other => panic!("{other:?}"),
    };
    assert!(login.credentials_path(&home).is_file());
    let account = provider.account_at(&home).await.unwrap().unwrap();
    assert_eq!(account.owner, CredentialOwner::Headroom);
    assert!(provider.fetch_limits(&account).await.is_ok());
    assert!(provider.usage_homes().await.unwrap().is_empty());
    let mut cursors = LogCursors::default();
    assert!(provider.read_usage(&home, &mut cursors).unwrap().is_empty());
}
