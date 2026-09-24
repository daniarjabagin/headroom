use std::collections::HashMap;

use headroom_core::account::{AccountId, CredentialOwner};
use headroom_core::quota::BalanceAmount;
use headroom_core::secret::SecretString;
use tempfile::TempDir;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::*;

const BALANCE: &str = include_str!("fixtures/current_balance.json");
const INVALID_KEY: &str = include_str!("fixtures/invalid_key.json");
const SECRET: &str = "poe-test-key";

struct Keys(HashMap<AccountId, SecretString>);

#[async_trait]
impl SecretReader for Keys {
    async fn read_secret(
        &self,
        account: &AccountId,
    ) -> Result<Option<SecretString>, ProviderError> {
        Ok(self.0.get(account).cloned())
    }
}

fn fixed_now() -> Timestamp {
    "2026-09-24T10:00:00Z".parse().unwrap()
}

async fn server(status: u16, body: &str) -> MockServer {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/usage/current_balance"))
        .and(header("authorization", format!("Bearer {SECRET}")))
        .respond_with(ResponseTemplate::new(status).set_body_string(body))
        .mount(&server)
        .await;
    server
}

fn setup(server: &MockServer, dir: &TempDir, stored: bool) -> (PoeProvider, AccountRef) {
    let account = AccountRef {
        id: identity(SECRET).account_id(&ID),
        provider: ID,
        home: dir.path().join("accounts/poe/a"),
        owner: CredentialOwner::Headroom,
    };
    let keys = if stored {
        HashMap::from([(account.id.clone(), SecretString::new(SECRET.into()))])
    } else {
        HashMap::new()
    };
    let config = PoeConfig {
        accounts_dir: dir.path().join("accounts/poe"),
        api_base: server.uri(),
    };
    let provider = PoeProvider::new(config, crate::http::client().unwrap(), Arc::new(Keys(keys)))
        .with_clock(fixed_now);
    (provider, account)
}

#[tokio::test]
async fn a_key_reports_its_point_balance() {
    let dir = tempfile::tempdir().unwrap();
    let server = server(200, BALANCE).await;
    let (provider, account) = setup(&server, &dir, true);
    let snapshot = provider.fetch_limits(&account).await.unwrap();
    assert_eq!(snapshot.identity.account_id(&ID), account.id);
    assert_eq!(snapshot.fetched_at, fixed_now());
    assert_eq!(snapshot.source, LimitsSource::Live);
    assert_eq!(snapshot.windows, []);
    assert_eq!(snapshot.notices, []);
    assert_eq!(
        snapshot.balances[0].amount,
        BalanceAmount::Count {
            value: 842_150,
            unit: "points".into()
        }
    );
}

#[tokio::test]
async fn a_revoked_key_is_signed_out_and_a_missing_key_is_not_signed_in() {
    let dir = tempfile::tempdir().unwrap();
    let server = server(401, INVALID_KEY).await;
    let (provider, account) = setup(&server, &dir, true);
    assert_eq!(
        provider.fetch_limits(&account).await,
        Err(ProviderError::SignInExpired)
    );
    let (provider, account) = setup(&server, &dir, false);
    assert_eq!(
        provider.fetch_limits(&account).await,
        Err(ProviderError::NotSignedIn)
    );
}

#[tokio::test]
async fn validation_names_the_key_by_its_hash_or_rejects_it_clearly() {
    let dir = tempfile::tempdir().unwrap();
    let server_ok = server(200, BALANCE).await;
    let (provider, account) = setup(&server_ok, &dir, false);
    let identity = provider.validate_key(SECRET).await.unwrap();
    assert_eq!(identity.account_id(&ID), account.id);
    assert!(identity.stable_key.starts_with("key-sha256:"));
    assert!(!identity.stable_key.contains(SECRET));
    let server_bad = server(401, INVALID_KEY).await;
    let (provider, _) = setup(&server_bad, &dir, false);
    assert_eq!(
        provider.validate_key(SECRET).await.unwrap_err().to_string(),
        "Poe rejected this API key; check it at https://poe.com/api/keys"
    );
}

#[tokio::test]
async fn discovery_lists_stored_key_accounts() {
    let dir = tempfile::tempdir().unwrap();
    let server = server(200, BALANCE).await;
    let (provider, account) = setup(&server, &dir, true);
    assert_eq!(provider.discover().await, Ok(Vec::new()));
    std::fs::create_dir_all(&account.home).unwrap();
    key_accounts::save_record(&account.home, &identity(SECRET)).unwrap();
    assert_eq!(provider.discover().await, Ok(vec![account]));
    assert_eq!(provider.usage_homes().await, Ok(Vec::new()));
}
