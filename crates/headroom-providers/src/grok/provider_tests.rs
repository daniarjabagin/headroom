use std::fs;

use headroom_core::account::{AccountId, AccountIdentity};
use headroom_core::quota::WindowId;
use serde_json::{Value, json};
use tempfile::TempDir;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::*;

const WEEKLY: &str = include_str!("fixtures/billing_weekly.json");
const SETTINGS: &str = include_str!("fixtures/settings.json");
const AUTH: &str = include_str!("fixtures/auth.json");

fn fixed_now() -> Timestamp {
    "2026-09-22T12:00:00Z".parse().unwrap()
}

fn late_now() -> Timestamp {
    "2026-09-23T10:00:00Z".parse().unwrap()
}

struct Setup {
    root: TempDir,
    config: GrokConfig,
}

impl Setup {
    fn new(server: &MockServer) -> Setup {
        let root = tempfile::tempdir().unwrap();
        let config = GrokConfig {
            xdg_data_home: root.path().join("data"),
            api_base: format!("{}/v1", server.uri()),
            issuer: server.uri(),
            ..GrokConfig::for_home(root.path().join("home"))
        };
        Setup { root, config }
    }

    fn cli_home(&self) -> PathBuf {
        self.root.path().join("home/.grok")
    }

    fn headroom_home(&self, name: &str) -> PathBuf {
        self.root
            .path()
            .join("data/headroom/accounts/grok")
            .join(name)
    }

    fn provider(&self, clock: Clock) -> GrokProvider {
        GrokProvider::with_clock(self.config.clone(), clock, reqwest::Client::new())
    }
}

fn sign_in(home: &Path, issuer: &str) {
    let mut document: Value = serde_json::from_str(AUTH).unwrap();
    let entry = document
        .as_object_mut()
        .unwrap()
        .values_mut()
        .next()
        .unwrap();
    entry["oidc_issuer"] = json!(issuer);
    fs::create_dir_all(home).unwrap();
    fs::write(home.join("auth.json"), document.to_string()).unwrap();
}

async fn mount_limits(server: &MockServer, token: &str, calls: u64) {
    for (route, body) in [("/v1/billing", WEEKLY), ("/v1/settings", SETTINGS)] {
        Mock::given(method("GET"))
            .and(path(route))
            .and(header("authorization", format!("Bearer {token}")))
            .respond_with(ResponseTemplate::new(200).set_body_string(body))
            .expect(calls)
            .mount(server)
            .await;
    }
}

async fn account(provider: &GrokProvider, home: &Path) -> AccountRef {
    let accounts = provider.discover().await.unwrap();
    accounts.into_iter().find(|a| a.home == home).unwrap()
}

#[tokio::test]
async fn a_live_cli_sign_in_shows_the_weekly_pool() {
    let server = MockServer::start().await;
    mount_limits(&server, "fake-access-token", 1).await;
    let setup = Setup::new(&server);
    sign_in(&setup.cli_home(), "https://auth.x.ai");
    let provider = setup.provider(fixed_now);
    let account = account(&provider, &setup.cli_home()).await;
    assert_eq!(account.owner, CredentialOwner::Cli);
    let snapshot = provider.fetch_limits(&account).await.unwrap();
    assert_eq!(snapshot.windows[0].id, WindowId::Weekly);
    assert_eq!(snapshot.identity.plan.as_deref(), Some("SuperGrok"));
}

#[tokio::test]
async fn an_expired_cli_token_is_never_refreshed() {
    let server = MockServer::start().await;
    mount_limits(&server, "fake-access-token", 0).await;
    let setup = Setup::new(&server);
    sign_in(&setup.cli_home(), &server.uri());
    let before = fs::read(setup.cli_home().join("auth.json")).unwrap();
    let provider = setup.provider(late_now);
    let account = account(&provider, &setup.cli_home()).await;
    assert_eq!(
        provider.fetch_limits(&account).await,
        Err(ProviderError::SignInExpired)
    );
    assert_eq!(
        fs::read(setup.cli_home().join("auth.json")).unwrap(),
        before
    );
    assert!(!setup.cli_home().join("auth.json.lock").exists());
}

#[tokio::test]
async fn an_expired_headroom_token_is_refreshed_and_saved() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/oauth2/token"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            r#"{"access_token":"new-access","refresh_token":"new-refresh","expires_in":21600}"#,
        ))
        .expect(1)
        .mount(&server)
        .await;
    mount_limits(&server, "new-access", 1).await;
    let setup = Setup::new(&server);
    let home = setup.headroom_home("one");
    sign_in(&home, &server.uri());
    let provider = setup.provider(late_now);
    let account = account(&provider, &home).await;
    assert_eq!(account.owner, CredentialOwner::Headroom);
    assert!(provider.fetch_limits(&account).await.is_ok());
    let saved = auth::load_credentials(&home).unwrap();
    assert_eq!(saved.access_token, "new-access");
    assert_eq!(saved.refresh_token.as_deref(), Some("new-refresh"));
}

#[tokio::test]
async fn a_rejected_headroom_token_is_refreshed_once_and_retried() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/billing"))
        .and(header("authorization", "Bearer fake-access-token"))
        .respond_with(ResponseTemplate::new(401))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/oauth2/token"))
        .respond_with(
            ResponseTemplate::new(200).set_body_string(r#"{"access_token":"new-access"}"#),
        )
        .expect(1)
        .mount(&server)
        .await;
    mount_limits(&server, "new-access", 1).await;
    let setup = Setup::new(&server);
    let home = setup.headroom_home("one");
    sign_in(&home, &server.uri());
    let provider = setup.provider(fixed_now);
    let account = account(&provider, &home).await;
    assert!(provider.fetch_limits(&account).await.is_ok());
}

#[tokio::test]
async fn the_same_account_prefers_its_headroom_home() {
    let server = MockServer::start().await;
    let setup = Setup::new(&server);
    sign_in(&setup.cli_home(), "https://auth.x.ai");
    sign_in(&setup.headroom_home("one"), "https://auth.x.ai");
    let provider = setup.provider(fixed_now);
    let accounts = provider.discover().await.unwrap();
    assert_eq!(accounts.len(), 1);
    assert_eq!(accounts[0].owner, CredentialOwner::Headroom);
    let id = AccountIdentity {
        email: None,
        plan: None,
        stable_key: "user-fake-1/team-fake-1".into(),
    }
    .account_id(&ID);
    assert_eq!(accounts[0].id, id);
}

#[tokio::test]
async fn nobody_signed_in_is_reported_and_account_at_finds_new_homes() {
    let server = MockServer::start().await;
    let setup = Setup::new(&server);
    let provider = setup.provider(fixed_now);
    assert_eq!(provider.discover().await, Err(ProviderError::NotSignedIn));
    let home = setup.headroom_home("new");
    assert_eq!(provider.account_at(&home).await, Ok(None));
    sign_in(&home, "https://auth.x.ai");
    let found = provider.account_at(&home).await.unwrap().unwrap();
    assert_eq!(found.owner, CredentialOwner::Headroom);
    assert_eq!(found.provider, ID);
}

#[tokio::test]
async fn a_changed_account_at_a_home_is_refused() {
    let server = MockServer::start().await;
    let setup = Setup::new(&server);
    sign_in(&setup.cli_home(), "https://auth.x.ai");
    let provider = setup.provider(fixed_now);
    let mut account = account(&provider, &setup.cli_home()).await;
    account.id = AccountId("grok:000000000000".into());
    assert!(matches!(
        provider.fetch_limits(&account).await,
        Err(ProviderError::LocalData(_))
    ));
}

#[tokio::test]
async fn usage_is_read_from_session_logs() {
    let server = MockServer::start().await;
    let setup = Setup::new(&server);
    let file = setup
        .cli_home()
        .join("sessions/%2Ftmp/session-1/updates.jsonl");
    fs::create_dir_all(file.parent().unwrap()).unwrap();
    fs::write(&file, include_str!("fixtures/updates.jsonl")).unwrap();
    let provider = setup.provider(fixed_now);
    assert_eq!(provider.usage_homes().await.unwrap(), [setup.cli_home()]);
    let events = provider
        .read_usage(&setup.cli_home(), &mut LogCursors::default())
        .unwrap();
    assert_eq!(events.len(), 2);
    assert!(events.iter().all(|event| event.reported_cost.is_some()));
}
