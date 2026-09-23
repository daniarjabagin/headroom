use std::collections::HashMap;
use std::fs;

use async_trait::async_trait;
use headroom_core::account::{AccountId, CredentialOwner};
use headroom_core::quota::WindowId;
use headroom_core::secret::SecretString;
use tempfile::TempDir;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::*;

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

async fn server_with(status: u16, body: &str) -> MockServer {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/token_plan/remains"))
        .and(header("authorization", "Bearer mm-key"))
        .respond_with(ResponseTemplate::new(status).set_body_string(body))
        .mount(&server)
        .await;
    server
}

fn provider(root: &TempDir, server: &MockServer, secrets: Secrets) -> MiniMaxProvider {
    let config = MiniMaxConfig {
        accounts_dir: root.path().join("minimax"),
        api_base: server.uri(),
    };
    MiniMaxProvider::with_clock(config, reqwest::Client::new(), Arc::new(secrets), fixed_now)
}

fn add_account(root: &TempDir, key: &str) -> (AccountRef, Secrets) {
    let identity = key_identity(key);
    let home = root.path().join("minimax/one");
    fs::create_dir_all(&home).unwrap();
    key_accounts::save_record(&home, &identity).unwrap();
    let id = identity.account_id(&ID);
    let account = AccountRef {
        id: id.clone(),
        provider: ID,
        home,
        owner: CredentialOwner::Headroom,
    };
    let secrets = Secrets(HashMap::from([(id, SecretString::new(key.into()))]));
    (account, secrets)
}

#[tokio::test]
async fn a_valid_key_names_a_stable_account_without_storing_the_key() {
    let server = server_with(200, include_str!("fixtures/remains_ok.json")).await;
    let root = tempfile::tempdir().unwrap();
    let provider = provider(&root, &server, Secrets(HashMap::new()));
    let identity = provider.validate_key("mm-key").await.unwrap();
    assert_eq!(identity, key_identity("mm-key"));
    assert_eq!(identity.plan.as_deref(), Some("Token Plan"));
    assert!(identity.stable_key.starts_with("key:"));
    assert!(!identity.stable_key.contains("mm-key"));
    assert_ne!(key_identity("mm-other"), identity);
}

#[tokio::test]
async fn rejected_keys_are_not_accepted() {
    let rejected = ProviderError::Unsupported(
        "MiniMax rejected this API key; check it at \
         https://platform.minimax.io/user-center/payment/token-plan"
            .into(),
    );
    for (status, body, expected) in [
        (
            200,
            include_str!("fixtures/invalid_key.json"),
            rejected.clone(),
        ),
        (
            200,
            include_str!("fixtures/no_plan.json"),
            ProviderError::NoSubscription {
                detail: mapper::NO_PLAN.into(),
            },
        ),
        (401, "", rejected.clone()),
    ] {
        let server = server_with(status, body).await;
        let root = tempfile::tempdir().unwrap();
        let provider = provider(&root, &server, Secrets(HashMap::new()));
        assert_eq!(provider.validate_key("mm-key").await.unwrap_err(), expected);
    }
}

#[tokio::test]
async fn stored_accounts_are_discovered_and_fetched_with_their_key() {
    let server = server_with(200, include_str!("fixtures/remains_boost.json")).await;
    let root = tempfile::tempdir().unwrap();
    let (account, secrets) = add_account(&root, "mm-key");
    let provider = provider(&root, &server, secrets);
    assert_eq!(
        provider.discover().await.unwrap(),
        std::slice::from_ref(&account)
    );
    let snapshot = provider.fetch_limits(&account).await.unwrap();
    assert_eq!(snapshot.identity, key_identity("mm-key"));
    assert_eq!(snapshot.fetched_at, fixed_now());
    assert_eq!(snapshot.source, LimitsSource::Live);
    let windows: Vec<_> = snapshot
        .windows
        .iter()
        .map(|w| (w.id.clone(), w.used.value()))
        .collect();
    assert_eq!(
        windows,
        [(WindowId::Session, 62.5), (WindowId::Weekly, 20.0)]
    );
    assert!(provider.usage_homes().await.unwrap().is_empty());
}

#[tokio::test]
async fn a_missing_key_or_record_means_signed_out() {
    let server = server_with(200, include_str!("fixtures/remains_ok.json")).await;
    let root = tempfile::tempdir().unwrap();
    let (account, _) = add_account(&root, "mm-key");
    let provider = provider(&root, &server, Secrets(HashMap::new()));
    assert_eq!(
        provider.fetch_limits(&account).await.unwrap_err(),
        ProviderError::NotSignedIn
    );
    let gone = AccountRef {
        home: root.path().join("minimax/missing"),
        ..account
    };
    assert_eq!(
        provider.fetch_limits(&gone).await.unwrap_err(),
        ProviderError::NotSignedIn
    );
}
