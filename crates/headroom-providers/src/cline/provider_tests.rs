use std::fs;
use std::time::Duration;

use headroom_core::quota::BalanceAmount;
use headroom_core::units::MicroUsd;
use serde_json::{Value, json};
use tempfile::TempDir;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::*;

const SIGNED_IN: &str = include_str!("fixtures/providers.json");
const ME: &str = include_str!("fixtures/me.json");
const ME_ORG: &str = include_str!("fixtures/me_org.json");
const PLAN: &str = include_str!("fixtures/plan_active.json");
const BALANCE: &str = include_str!("fixtures/balance.json");
const ORG_BALANCE: &str = include_str!("fixtures/org_balance.json");
const REFRESH: &str = include_str!("fixtures/refresh.json");

fn fixed_now() -> Timestamp {
    "2026-09-23T10:00:00Z".parse().unwrap()
}

fn after_expiry() -> Timestamp {
    "2026-09-23T10:45:00Z".parse().unwrap()
}

fn sign_in(providers_file: &Path, user: &str) {
    let mut file: Value = serde_json::from_str(SIGNED_IN).unwrap();
    file["providers"]["cline"]["settings"]["auth"]["accountId"] = json!(user);
    fs::create_dir_all(providers_file.parent().unwrap()).unwrap();
    fs::write(providers_file, serde_json::to_string_pretty(&file).unwrap()).unwrap();
}

fn cli_file(home: &TempDir) -> PathBuf {
    home.path().join(".cline/data/settings/providers.json")
}

fn owned_home(home: &TempDir, name: &str) -> PathBuf {
    ClineConfig::for_home(home.path().to_path_buf())
        .headroom_accounts_dir()
        .join(name)
}

fn provider(home: &TempDir, server: &MockServer, clock: Clock) -> ClineProvider {
    let mut config = ClineConfig::for_home(home.path().to_path_buf());
    config.api_base = server.uri();
    ClineProvider::with_clock(config, clock).unwrap()
}

async fn mount(server: &MockServer, route: &str, body: &str, bearer: &str) {
    Mock::given(method("GET"))
        .and(path(route))
        .and(header("authorization", format!("Bearer {bearer}").as_str()))
        .respond_with(ResponseTemplate::new(200).set_body_string(body))
        .expect(1)
        .mount(server)
        .await;
}

async fn account_server(me: &str, bearer: &str) -> MockServer {
    let server = MockServer::start().await;
    mount(&server, "/api/v1/users/me", me, bearer).await;
    mount(&server, "/api/v1/users/me/plan", PLAN, bearer).await;
    mount(
        &server,
        "/api/v1/users/usr-0000000000000001/balance",
        BALANCE,
        bearer,
    )
    .await;
    server
}

fn usd(snapshot: &LimitsSnapshot) -> Vec<(String, i64)> {
    snapshot
        .balances
        .iter()
        .map(|balance| match balance.amount {
            BalanceAmount::Usd(MicroUsd(value)) => (balance.id.clone(), value),
            BalanceAmount::Count { .. } | BalanceAmount::Money(_) => panic!("{balance:?}"),
        })
        .collect()
}

#[tokio::test]
async fn cli_and_headroom_homes_are_discovered_cli_first() {
    let home = tempfile::tempdir().unwrap();
    sign_in(&cli_file(&home), "usr-0000000000000001");
    let same = owned_home(&home, "a");
    sign_in(
        &same.join("data/settings/providers.json"),
        "usr-0000000000000001",
    );
    let other = owned_home(&home, "b");
    sign_in(&other.join("data/settings/providers.json"), "usr-2");
    fs::create_dir_all(owned_home(&home, "empty")).unwrap();
    let server = MockServer::start().await;
    let provider = provider(&home, &server, fixed_now);
    let accounts = provider.discover().await.unwrap();
    let found: Vec<(&Path, CredentialOwner)> = accounts
        .iter()
        .map(|account| (account.home.as_path(), account.owner))
        .collect();
    assert_eq!(
        found,
        [
            (home.path().join(".cline").as_path(), CredentialOwner::Cli),
            (same.as_path(), CredentialOwner::Headroom),
            (other.as_path(), CredentialOwner::Headroom)
        ]
    );
    assert!(accounts[0].id.0.starts_with("cline:"));
    let at = provider.account_at(&same).await.unwrap().unwrap();
    assert_eq!(
        (at.id.clone(), at.owner),
        (accounts[0].id.clone(), CredentialOwner::Headroom)
    );
    assert!(
        provider
            .account_at(&owned_home(&home, "empty"))
            .await
            .unwrap()
            .is_none()
    );
    assert!(provider.usage_homes().await.unwrap().is_empty());
}

#[tokio::test]
async fn a_fresh_cli_sign_in_fetches_plan_and_credits() {
    let home = tempfile::tempdir().unwrap();
    sign_in(&cli_file(&home), "usr-0000000000000001");
    let server = account_server(ME, "workos:fake.jwt.token").await;
    let provider = provider(&home, &server, fixed_now);
    let account = provider.discover().await.unwrap().remove(0);
    let snapshot = provider.fetch_limits(&account).await.unwrap();
    assert_eq!(snapshot.identity.plan.as_deref(), Some("ClinePass Pro"));
    assert_eq!(
        snapshot.identity.email.as_deref(),
        Some("someone@example.com")
    );
    assert_eq!(snapshot.identity.stable_key, "usr-0000000000000001");
    assert!(snapshot.windows.is_empty());
    assert_eq!(usd(&snapshot), [("credits".to_owned(), 12_345_678)]);
    assert_eq!(snapshot.source, LimitsSource::Live);
    assert_eq!(snapshot.fetched_at, fixed_now());
}

#[tokio::test]
async fn an_active_organization_adds_its_balance() {
    let home = tempfile::tempdir().unwrap();
    sign_in(&cli_file(&home), "usr-0000000000000001");
    let server = account_server(ME_ORG, "workos:fake.jwt.token").await;
    mount(
        &server,
        "/api/v1/organizations/org-0000000000000001/balance",
        ORG_BALANCE,
        "workos:fake.jwt.token",
    )
    .await;
    let provider = provider(&home, &server, fixed_now);
    let account = provider.discover().await.unwrap().remove(0);
    let snapshot = provider.fetch_limits(&account).await.unwrap();
    assert_eq!(
        usd(&snapshot),
        [
            ("organization_credits".to_owned(), 250_000_000),
            ("credits".to_owned(), 12_345_678)
        ]
    );
}

#[tokio::test]
async fn an_expired_cli_sign_in_is_never_refreshed() {
    let home = tempfile::tempdir().unwrap();
    sign_in(&cli_file(&home), "usr-0000000000000001");
    let server = MockServer::start().await;
    Mock::given(wiremock::matchers::any())
        .respond_with(ResponseTemplate::new(500))
        .expect(0)
        .mount(&server)
        .await;
    let provider = provider(&home, &server, after_expiry);
    let account = provider.discover().await.unwrap().remove(0);
    assert_eq!(
        provider.fetch_limits(&account).await.unwrap_err(),
        ProviderError::SignInExpired
    );
    assert_eq!(fs::read_to_string(cli_file(&home)).unwrap(), {
        let mut file: Value = serde_json::from_str(SIGNED_IN).unwrap();
        file["providers"]["cline"]["settings"]["auth"]["accountId"] = json!("usr-0000000000000001");
        serde_json::to_string_pretty(&file).unwrap()
    });
}

#[tokio::test]
async fn an_expired_headroom_sign_in_is_refreshed_and_saved() {
    let home = tempfile::tempdir().unwrap();
    let owned = owned_home(&home, "a");
    let file = owned.join("data/settings/providers.json");
    sign_in(&file, "usr-0000000000000001");
    let server = account_server(ME, "workos:fresh.jwt.token").await;
    Mock::given(method("POST"))
        .and(path("/api/v1/auth/refresh"))
        .respond_with(ResponseTemplate::new(200).set_body_string(REFRESH))
        .expect(1)
        .mount(&server)
        .await;
    let provider = provider(&home, &server, after_expiry);
    let account = provider.account_at(&owned).await.unwrap().unwrap();
    let snapshot = provider.fetch_limits(&account).await.unwrap();
    assert_eq!(usd(&snapshot), [("credits".to_owned(), 12_345_678)]);
    let saved = auth::parse_credentials(&fs::read_to_string(&file).unwrap()).unwrap();
    assert_eq!(saved.access_token.expose(), "workos:fresh.jwt.token");
    assert_eq!(saved.refresh_token.unwrap().expose(), "rotated-refresh");
}

#[tokio::test]
async fn a_refresh_is_saved_even_when_the_fetch_is_abandoned() {
    let home = tempfile::tempdir().unwrap();
    let owned = owned_home(&home, "a");
    let file = owned.join("data/settings/providers.json");
    sign_in(&file, "usr-0000000000000001");
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/v1/auth/refresh"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(REFRESH)
                .set_delay(Duration::from_millis(200)),
        )
        .expect(1)
        .mount(&server)
        .await;
    let provider = provider(&home, &server, after_expiry);
    let account = provider.account_at(&owned).await.unwrap().unwrap();
    let abandoned =
        tokio::time::timeout(Duration::from_millis(50), provider.fetch_limits(&account)).await;
    assert!(abandoned.is_err());
    for _ in 0..300 {
        let saved = auth::parse_credentials(&fs::read_to_string(&file).unwrap()).unwrap();
        if saved.access_token.expose() == "workos:fresh.jwt.token" {
            return;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("the refreshed sign-in was never saved");
}

#[tokio::test]
async fn a_rejected_refresh_means_sign_in_expired() {
    let home = tempfile::tempdir().unwrap();
    let owned = owned_home(&home, "a");
    let file = owned.join("data/settings/providers.json");
    sign_in(&file, "usr-0000000000000001");
    let before = fs::read_to_string(&file).unwrap();
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/v1/auth/refresh"))
        .respond_with(ResponseTemplate::new(401))
        .mount(&server)
        .await;
    let provider = provider(&home, &server, after_expiry);
    let account = provider.account_at(&owned).await.unwrap().unwrap();
    assert_eq!(
        provider.fetch_limits(&account).await.unwrap_err(),
        ProviderError::SignInExpired
    );
    assert_eq!(fs::read_to_string(&file).unwrap(), before);
}

#[tokio::test]
async fn a_different_account_in_the_same_home_is_an_error() {
    let home = tempfile::tempdir().unwrap();
    sign_in(&cli_file(&home), "usr-0000000000000001");
    let server = MockServer::start().await;
    let provider = provider(&home, &server, fixed_now);
    let account = provider.discover().await.unwrap().remove(0);
    sign_in(&cli_file(&home), "usr-2");
    assert!(matches!(
        provider.fetch_limits(&account).await,
        Err(ProviderError::LocalData(_))
    ));
    fs::remove_file(cli_file(&home)).unwrap();
    assert_eq!(
        provider.fetch_limits(&account).await.unwrap_err(),
        ProviderError::NotSignedIn
    );
}
