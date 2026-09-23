use std::collections::HashMap;
use std::fs;

use headroom_core::account::{AccountId, CredentialOwner};
use headroom_core::quota::WindowId;
use jiff::SignedDuration;
use serde_json::{Value, json};
use tempfile::TempDir;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::*;
use crate::key_accounts;

struct Secrets(HashMap<AccountId, SecretString>);

#[async_trait]
impl SecretReader for Secrets {
    async fn read_secret(
        &self,
        account: &AccountId,
    ) -> Result<Option<SecretString>, ProviderError> {
        Ok(self.0.get(account).cloned())
    }
}

fn fixed_now() -> Timestamp {
    "2026-09-23T10:00:00Z".parse().unwrap()
}

async fn usages_server(bearer: &str, body: &str) -> MockServer {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/coding/v1/usages"))
        .and(header("authorization", format!("Bearer {bearer}").as_str()))
        .respond_with(ResponseTemplate::new(200).set_body_string(body))
        .mount(&server)
        .await;
    server
}

fn provider(root: &TempDir, server: &MockServer, secrets: Secrets) -> KimiProvider {
    let config = KimiConfig {
        share_dir: root.path().join(".kimi"),
        accounts_dir: root.path().join("accounts/kimi"),
        api_base: format!("{}/coding/v1", server.uri()),
        oauth_host: server.uri(),
    };
    KimiProvider::with_clock(config, reqwest::Client::new(), Arc::new(secrets), fixed_now)
}

fn write_tokens(home: &Path, expires_at: Timestamp) {
    let path = home.join(credentials::CREDENTIALS_FILE);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    let tokens = json!({
        "access_token": "fake-access-token",
        "refresh_token": "fake-refresh-token",
        "expires_at": expires_at.as_second(),
        "scope": "kimi-code",
        "token_type": "Bearer",
        "expires_in": 900
    });
    fs::write(path, tokens.to_string()).unwrap();
}

async fn oauth_account(provider: &KimiProvider, home: &Path) -> AccountRef {
    provider
        .discover()
        .await
        .unwrap()
        .into_iter()
        .find(|account| account.home == home)
        .unwrap()
}

#[tokio::test]
async fn a_valid_key_is_identified_by_its_fingerprint_and_plan() {
    let server = usages_server("sk-kimi", include_str!("fixtures/usages_counts.json")).await;
    let root = tempfile::tempdir().unwrap();
    let provider = provider(&root, &server, Secrets(HashMap::new()));
    let identity = provider.validate_key("sk-kimi").await.unwrap();
    assert_eq!(
        identity,
        accounts::key_identity("sk-kimi", Some("Basic".into()))
    );
    let error = provider.validate_key("sk-wrong").await.unwrap_err();
    assert_eq!(
        error,
        ProviderError::NoSubscription {
            detail: client::NO_PLAN.into()
        }
    );
}

#[tokio::test]
async fn a_rejected_key_points_to_the_console() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(401))
        .mount(&server)
        .await;
    let root = tempfile::tempdir().unwrap();
    let provider = provider(&root, &server, Secrets(HashMap::new()));
    assert_eq!(
        provider.validate_key("sk-bad").await.unwrap_err(),
        ProviderError::Unsupported(
            "Kimi Code rejected this API key; check it at https://www.kimi.com/code/console".into()
        )
    );
}

#[tokio::test]
async fn key_accounts_fetch_with_the_stored_key() {
    let server = usages_server("sk-kimi", include_str!("fixtures/usages_counts.json")).await;
    let root = tempfile::tempdir().unwrap();
    let identity = accounts::key_identity("sk-kimi", None);
    let home = root.path().join("accounts/kimi/one");
    fs::create_dir_all(&home).unwrap();
    key_accounts::save_record(&home, &identity).unwrap();
    let id = identity.account_id(&ID);
    let secrets = Secrets(HashMap::from([(
        id.clone(),
        SecretString::new("sk-kimi".into()),
    )]));
    let keyed = provider(&root, &server, secrets);
    let account = keyed.discover().await.unwrap().remove(0);
    assert_eq!(
        (account.id.clone(), account.owner),
        (id, CredentialOwner::Headroom)
    );
    let snapshot = keyed.fetch_limits(&account).await.unwrap();
    assert_eq!(snapshot.identity.plan.as_deref(), Some("Basic"));
    assert_eq!(snapshot.identity.stable_key, identity.stable_key);
    let windows: Vec<_> = snapshot
        .windows
        .iter()
        .map(|w| (w.id.clone(), w.used.value()))
        .collect();
    assert_eq!(
        windows,
        [(WindowId::Session, 20.0), (WindowId::Weekly, 25.0)]
    );
    assert_eq!(snapshot.fetched_at, fixed_now());

    let unkeyed = provider(&root, &server, Secrets(HashMap::new()));
    assert_eq!(
        unkeyed.fetch_limits(&account).await.unwrap_err(),
        ProviderError::NotSignedIn
    );
}

#[tokio::test]
async fn a_fresh_cli_token_is_used_read_only() {
    let server = usages_server(
        "fake-access-token",
        include_str!("fixtures/usages_wrapped.json"),
    )
    .await;
    let root = tempfile::tempdir().unwrap();
    let share = root.path().join(".kimi");
    write_tokens(&share, fixed_now() + SignedDuration::from_mins(2));
    let before = fs::read(share.join(credentials::CREDENTIALS_FILE)).unwrap();
    let provider = provider(&root, &server, Secrets(HashMap::new()));
    let account = oauth_account(&provider, &share).await;
    assert_eq!(account.owner, CredentialOwner::Cli);
    let snapshot = provider.fetch_limits(&account).await.unwrap();
    assert_eq!(snapshot.windows.len(), 2);
    assert_eq!(snapshot.identity.plan, None);
    assert_eq!(
        fs::read(share.join(credentials::CREDENTIALS_FILE)).unwrap(),
        before
    );
}

#[tokio::test]
async fn an_expired_cli_token_asks_for_the_cli() {
    let server = MockServer::start().await;
    let root = tempfile::tempdir().unwrap();
    let share = root.path().join(".kimi");
    write_tokens(&share, fixed_now() - SignedDuration::from_mins(1));
    let provider = provider(&root, &server, Secrets(HashMap::new()));
    let account = oauth_account(&provider, &share).await;
    assert_eq!(
        provider.fetch_limits(&account).await.unwrap_err(),
        ProviderError::SignInExpired
    );
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn an_expiring_headroom_token_is_refreshed_and_saved() {
    let server = usages_server(
        "fake-access-token-2",
        include_str!("fixtures/usages_counts.json"),
    )
    .await;
    Mock::given(method("POST"))
        .and(path("/api/oauth/token"))
        .respond_with(
            ResponseTemplate::new(200).set_body_string(include_str!("fixtures/refreshed.json")),
        )
        .expect(1)
        .mount(&server)
        .await;
    let root = tempfile::tempdir().unwrap();
    let home = root.path().join("accounts/kimi/login");
    write_tokens(&home, fixed_now() + SignedDuration::from_mins(2));
    let provider = provider(&root, &server, Secrets(HashMap::new()));
    let account = oauth_account(&provider, &home).await;
    assert_eq!(account.owner, CredentialOwner::Headroom);
    let snapshot = provider.fetch_limits(&account).await.unwrap();
    assert_eq!(snapshot.identity.plan.as_deref(), Some("Basic"));
    let saved: Value =
        serde_json::from_slice(&fs::read(home.join(credentials::CREDENTIALS_FILE)).unwrap())
            .unwrap();
    assert_eq!(saved["refresh_token"], "fake-refresh-token-2");
    assert_eq!(saved["expires_at"], fixed_now().as_second() + 900);
}

#[test]
fn the_descriptor_prefers_keys_and_offers_the_cli_login() {
    assert_eq!(DESCRIPTOR.validate(), Ok(()));
    assert!(matches!(
        DESCRIPTOR.default_method(),
        Some(AddAccountMethod::ApiKey(_))
    ));
    match DESCRIPTOR.add_account.get(1) {
        Some(AddAccountMethod::CliLogin(login)) => {
            assert_eq!(login.program, "kimi");
            assert_eq!(
                login.credentials_path(Path::new("/h")),
                Path::new("/h/credentials/kimi-code.json")
            );
        }
        other => panic!("{other:?}"),
    }
}
