use headroom_core::quota::{BalanceAmount, WindowId};
use headroom_core::units::{MicroUsd, Percent};
use tempfile::TempDir;
use wiremock::matchers::{body_string, header, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::test_support::{config_in, fixed_now, sign_in_agent, sign_in_ide, token, valid_token};
use super::*;

const PRO: &str = include_str!("fixtures/period_usage_pro.json");
const FREE: &str = include_str!("fixtures/period_usage_free.json");
const REQUEST_BASED: &str = include_str!("fixtures/period_usage_request_based.json");
const PLAN_INFO: &str = include_str!("fixtures/plan_info.json");
const GRANTS: &str = include_str!("fixtures/credit_grants.json");
const STRIPE: &str = include_str!("fixtures/stripe.json");
const GROK_BOT: &str = include_str!("fixtures/grok_bot.json");
const REQUESTS: &str = include_str!("fixtures/request_usage.json");
const UNAUTHENTICATED: &str = include_str!("fixtures/unauthenticated.json");
const RATE_LIMITED: &str = include_str!("fixtures/rate_limited.json");
const SUBJECT: &str = "auth0|user_01";

async fn mount_connect(server: &MockServer, rpc: &str, status: u16, body: &str) {
    Mock::given(method("POST"))
        .and(path(format!("/aiserver.v1.DashboardService/{rpc}")))
        .and(header("Connect-Protocol-Version", "1"))
        .and(header("content-type", "application/json"))
        .and(body_string("{}"))
        .respond_with(ResponseTemplate::new(status).set_body_string(body))
        .mount(server)
        .await;
}

async fn mount_web(server: &MockServer, web_path: &str, status: u16, body: &str) {
    Mock::given(method("GET"))
        .and(path(web_path))
        .respond_with(ResponseTemplate::new(status).set_body_string(body))
        .mount(server)
        .await;
}

async fn full_server(usage: &str) -> MockServer {
    let server = MockServer::start().await;
    mount_connect(&server, "GetCurrentPeriodUsage", 200, usage).await;
    mount_connect(&server, "GetPlanInfo", 200, PLAN_INFO).await;
    mount_connect(&server, "GetCreditGrantsBalance", 200, GRANTS).await;
    mount_connect(&server, "GetSandUsageStatus", 200, GROK_BOT).await;
    mount_web(&server, "/api/auth/stripe", 200, STRIPE).await;
    server
}

fn provider(home: &TempDir, server: &MockServer) -> CursorProvider {
    let mut config = config_in(home.path());
    config.api_base = server.uri();
    config.web_base = server.uri();
    CursorProvider::with_clock(config, fixed_now).unwrap()
}

async fn discovered(provider: &CursorProvider) -> AccountRef {
    let mut accounts = provider.discover().await.unwrap();
    assert_eq!(accounts.len(), 1);
    accounts.remove(0)
}

fn signed_in_home() -> TempDir {
    let home = tempfile::tempdir().unwrap();
    sign_in_ide(&config_in(home.path()), &valid_token(SUBJECT), "pro");
    home
}

#[test]
fn descriptor_is_auto_detect_single_account_without_local_usage() {
    assert_eq!(DESCRIPTOR.validate(), Ok(()));
    assert!(matches!(
        DESCRIPTOR.default_method(),
        Some(AddAccountMethod::AutoDetect { .. })
    ));
    assert!(!DESCRIPTOR.multi_account);
    assert!(!DESCRIPTOR.local_usage);
}

#[tokio::test]
async fn discovery_finds_the_one_ide_login() {
    let home = signed_in_home();
    let server = MockServer::start().await;
    let provider = provider(&home, &server);
    let account = discovered(&provider).await;
    assert_eq!(account.id, AccountId::from_stable_key(&ID, SUBJECT));
    assert_eq!(account.provider, ID);
    assert_eq!(account.owner, CredentialOwner::Cli);
    assert_eq!(account.home, config_in(home.path()).ide_dir());
    assert!(provider.usage_homes().await.unwrap().is_empty());
}

#[tokio::test]
async fn nothing_signed_in_discovers_nothing() {
    let home = tempfile::tempdir().unwrap();
    let server = MockServer::start().await;
    assert!(
        provider(&home, &server)
            .discover()
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn pro_account_fetches_every_limit() {
    let home = signed_in_home();
    let server = full_server(PRO).await;
    let provider = provider(&home, &server);
    let snapshot = provider
        .fetch_limits(&discovered(&provider).await)
        .await
        .unwrap();
    assert_eq!(snapshot.identity.stable_key, SUBJECT);
    assert_eq!(snapshot.identity.plan.as_deref(), Some("Pro"));
    assert_eq!(
        snapshot.identity.email.as_deref(),
        Some("someone@example.com")
    );
    assert_eq!(snapshot.source, LimitsSource::Live);
    assert_eq!(snapshot.fetched_at, fixed_now());
    let windows: Vec<_> = snapshot
        .windows
        .iter()
        .map(|w| (w.id.clone(), w.used))
        .collect();
    assert_eq!(
        windows,
        [
            (WindowId::Other("total".into()), Percent::new(36.75)),
            (WindowId::Other("auto".into()), Percent::new(12.5)),
            (WindowId::Other("api".into()), Percent::new(60.25)),
            (WindowId::Other("grok_bot".into()), Percent::new(37.5)),
        ]
    );
    let balances: Vec<_> = snapshot
        .balances
        .iter()
        .map(|b| (b.id.as_str(), b.amount.clone()))
        .collect();
    assert_eq!(
        balances,
        [
            ("on_demand_spent", BalanceAmount::Usd(MicroUsd(12_340_000))),
            ("on_demand_limit", BalanceAmount::Usd(MicroUsd(50_000_000))),
            ("credits", BalanceAmount::Usd(MicroUsd(20_000_000))),
        ]
    );
}

#[tokio::test]
async fn requests_carry_the_bearer_token_and_session_cookie() {
    let home = tempfile::tempdir().unwrap();
    let secret = valid_token("google-oauth2|user_02");
    sign_in_ide(&config_in(home.path()), &secret, "pro");
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/aiserver.v1.DashboardService/GetCurrentPeriodUsage"))
        .and(header("authorization", format!("Bearer {secret}")))
        .respond_with(ResponseTemplate::new(200).set_body_string(REQUEST_BASED))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/usage"))
        .and(query_param("user", "user_02"))
        .and(header(
            "cookie",
            format!("WorkosCursorSessionToken=user_02%3A%3A{secret}"),
        ))
        .respond_with(ResponseTemplate::new(200).set_body_string(REQUESTS))
        .expect(1)
        .mount(&server)
        .await;
    let provider = provider(&home, &server);
    let snapshot = provider
        .fetch_limits(&discovered(&provider).await)
        .await
        .unwrap();
    assert_eq!(snapshot.windows.len(), 1);
    assert_eq!(snapshot.windows[0].id, WindowId::Other("requests".into()));
    assert_eq!(snapshot.identity.plan.as_deref(), Some("Pro"));
    assert!(snapshot.balances.is_empty());
}

#[tokio::test]
async fn optional_endpoint_failures_keep_the_primary_limits() {
    let home = signed_in_home();
    let server = MockServer::start().await;
    mount_connect(&server, "GetCurrentPeriodUsage", 200, PRO).await;
    mount_connect(&server, "GetPlanInfo", 500, "").await;
    mount_connect(&server, "GetCreditGrantsBalance", 200, "not json").await;
    mount_connect(&server, "GetSandUsageStatus", 429, RATE_LIMITED).await;
    mount_web(&server, "/api/auth/stripe", 401, UNAUTHENTICATED).await;
    let provider = provider(&home, &server);
    let snapshot = provider
        .fetch_limits(&discovered(&provider).await)
        .await
        .unwrap();
    assert_eq!(snapshot.windows.len(), 3);
    assert_eq!(snapshot.balances.len(), 2);
    assert_eq!(snapshot.identity.plan.as_deref(), Some("Pro"));
}

#[tokio::test]
async fn free_account_is_no_subscription() {
    let home = tempfile::tempdir().unwrap();
    sign_in_ide(&config_in(home.path()), &valid_token(SUBJECT), "free");
    let server = MockServer::start().await;
    mount_connect(&server, "GetCurrentPeriodUsage", 200, FREE).await;
    mount_connect(
        &server,
        "GetPlanInfo",
        200,
        r#"{"planInfo":{"planName":"Free"}}"#,
    )
    .await;
    let provider = provider(&home, &server);
    let error = provider
        .fetch_limits(&discovered(&provider).await)
        .await
        .unwrap_err();
    assert_eq!(
        error,
        ProviderError::NoSubscription {
            detail: "No active Cursor subscription (Free plan).".into()
        }
    );
}

#[tokio::test]
async fn expired_token_is_reported_without_calling_cursor() {
    let home = tempfile::tempdir().unwrap();
    let expired = token(SUBJECT, fixed_now() - jiff::SignedDuration::from_mins(1));
    sign_in_ide(&config_in(home.path()), &expired, "pro");
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_string(PRO))
        .expect(0)
        .mount(&server)
        .await;
    let provider = provider(&home, &server);
    let error = provider
        .fetch_limits(&discovered(&provider).await)
        .await
        .unwrap_err();
    assert_eq!(error, ProviderError::SignInExpired);
}

#[tokio::test]
async fn rejected_token_is_sign_in_expired() {
    let home = signed_in_home();
    let server = MockServer::start().await;
    mount_connect(&server, "GetCurrentPeriodUsage", 401, UNAUTHENTICATED).await;
    let provider = provider(&home, &server);
    let error = provider
        .fetch_limits(&discovered(&provider).await)
        .await
        .unwrap_err();
    assert_eq!(error, ProviderError::SignInExpired);
}

#[tokio::test]
async fn rate_limit_honours_retry_after() {
    let home = signed_in_home();
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/aiserver.v1.DashboardService/GetCurrentPeriodUsage"))
        .respond_with(
            ResponseTemplate::new(429)
                .insert_header("retry-after", "90")
                .set_body_string(RATE_LIMITED),
        )
        .mount(&server)
        .await;
    let provider = provider(&home, &server);
    let error = provider
        .fetch_limits(&discovered(&provider).await)
        .await
        .unwrap_err();
    assert_eq!(
        error,
        ProviderError::RateLimited {
            retry_after: Some(jiff::SignedDuration::from_secs(90))
        }
    );
}

#[tokio::test]
async fn request_based_plan_without_counts_is_an_invalid_response() {
    let home = signed_in_home();
    let server = MockServer::start().await;
    mount_connect(&server, "GetCurrentPeriodUsage", 200, REQUEST_BASED).await;
    mount_web(&server, "/api/usage", 200, r#"{"gpt-4":{"numRequests":3}}"#).await;
    let provider = provider(&home, &server);
    let error = provider
        .fetch_limits(&discovered(&provider).await)
        .await
        .unwrap_err();
    assert!(matches!(error, ProviderError::InvalidResponse(_)));
}

#[tokio::test]
async fn a_different_login_is_not_fetched_under_the_old_account() {
    let home = signed_in_home();
    let server = full_server(PRO).await;
    let provider = provider(&home, &server);
    let account = discovered(&provider).await;
    sign_in_ide(
        &config_in(home.path()),
        &valid_token("auth0|someone_else"),
        "pro",
    );
    let error = provider.fetch_limits(&account).await.unwrap_err();
    assert!(matches!(error, ProviderError::LocalData(_)));
}

#[tokio::test]
async fn agent_login_is_used_when_the_app_is_absent() {
    let home = tempfile::tempdir().unwrap();
    sign_in_agent(&config_in(home.path()), &valid_token(SUBJECT));
    let server = full_server(PRO).await;
    let provider = provider(&home, &server);
    let account = discovered(&provider).await;
    assert_eq!(account.home, home.path().join(".config/cursor"));
    let snapshot = provider.fetch_limits(&account).await.unwrap();
    assert_eq!(snapshot.identity.email, None);
    assert_eq!(snapshot.windows.len(), 4);
}

#[tokio::test]
async fn signed_out_after_discovery_is_not_signed_in() {
    let home = signed_in_home();
    let server = MockServer::start().await;
    let provider = provider(&home, &server);
    let account = discovered(&provider).await;
    std::fs::remove_file(config_in(home.path()).state_db()).unwrap();
    assert_eq!(
        provider.fetch_limits(&account).await.unwrap_err(),
        ProviderError::NotSignedIn
    );
}
