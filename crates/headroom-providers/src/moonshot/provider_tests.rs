use std::collections::HashMap;

use headroom_core::account::{AccountId, CredentialOwner};
use headroom_core::quota::BalanceAmount;
use headroom_core::secret::SecretString;
use headroom_core::units::{CurrencyCode, Money};
use tempfile::TempDir;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::*;

const BALANCE: &str = include_str!("fixtures/balance.json");
const INVALID_KEY: &str = include_str!("fixtures/invalid_key.json");
const SECRET: &str = "sk-moonshot-test";

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

async fn host(status: u16, body: &str) -> MockServer {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/users/me/balance"))
        .and(header("authorization", format!("Bearer {SECRET}")))
        .respond_with(ResponseTemplate::new(status).set_body_string(body))
        .mount(&server)
        .await;
    server
}

fn setup(
    global: &MockServer,
    mainland: &MockServer,
    dir: &TempDir,
    stored: bool,
) -> (MoonshotProvider, AccountRef) {
    let account = AccountRef {
        id: identity(SECRET).account_id(&ID),
        provider: ID,
        home: dir.path().join("accounts/moonshot/a"),
        owner: CredentialOwner::Headroom,
    };
    let keys = if stored {
        HashMap::from([(account.id.clone(), SecretString::new(SECRET.into()))])
    } else {
        HashMap::new()
    };
    let config = MoonshotConfig {
        accounts_dir: dir.path().join("accounts/moonshot"),
        global_api_base: global.uri(),
        mainland_api_base: mainland.uri(),
    };
    let provider =
        MoonshotProvider::new(config, crate::http::client().unwrap(), Arc::new(Keys(keys)))
            .with_clock(fixed_now);
    (provider, account)
}

fn available(snapshot: &LimitsSnapshot) -> &BalanceAmount {
    &snapshot.balances[0].amount
}

fn money(currency: &str, micros: i64) -> BalanceAmount {
    BalanceAmount::Money(Money {
        currency: CurrencyCode::parse(currency).unwrap(),
        micros,
    })
}

#[tokio::test]
async fn a_global_key_reports_dollars() {
    let dir = tempfile::tempdir().unwrap();
    let global = host(200, BALANCE).await;
    let mainland = host(401, INVALID_KEY).await;
    let (provider, account) = setup(&global, &mainland, &dir, true);
    let snapshot = provider.fetch_limits(&account).await.unwrap();
    assert_eq!(snapshot.identity.account_id(&ID), account.id);
    assert_eq!(snapshot.fetched_at, fixed_now());
    assert_eq!(snapshot.source, LimitsSource::Live);
    assert_eq!(snapshot.windows, []);
    assert_eq!(available(&snapshot), &money("USD", 49_588_940));
    assert!(mainland.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn a_mainland_key_reports_yuan_and_is_asked_there_first_next_time() {
    let dir = tempfile::tempdir().unwrap();
    let global = host(401, INVALID_KEY).await;
    let mainland = host(200, BALANCE).await;
    let (provider, account) = setup(&global, &mainland, &dir, true);
    let snapshot = provider.fetch_limits(&account).await.unwrap();
    assert_eq!(available(&snapshot), &money("CNY", 49_588_940));
    provider.fetch_limits(&account).await.unwrap();
    assert_eq!(global.received_requests().await.unwrap().len(), 1);
    assert_eq!(mainland.received_requests().await.unwrap().len(), 2);
}

#[tokio::test]
async fn a_revoked_key_is_signed_out_and_a_missing_key_is_not_signed_in() {
    let dir = tempfile::tempdir().unwrap();
    let global = host(401, INVALID_KEY).await;
    let mainland = host(401, INVALID_KEY).await;
    let (provider, account) = setup(&global, &mainland, &dir, true);
    assert_eq!(
        provider.fetch_limits(&account).await,
        Err(ProviderError::SignInExpired)
    );
    let (provider, account) = setup(&global, &mainland, &dir, false);
    assert_eq!(
        provider.fetch_limits(&account).await,
        Err(ProviderError::NotSignedIn)
    );
}

#[tokio::test]
async fn validation_accepts_a_key_from_either_region() {
    let dir = tempfile::tempdir().unwrap();
    let global = host(401, INVALID_KEY).await;
    let mainland = host(200, BALANCE).await;
    let (provider, account) = setup(&global, &mainland, &dir, false);
    let identity = provider.validate_key(SECRET).await.unwrap();
    assert_eq!(identity.account_id(&ID), account.id);
    assert!(identity.stable_key.starts_with("key-sha256:"));
    assert!(!identity.stable_key.contains(SECRET));
}

#[tokio::test]
async fn validation_rejects_an_invalid_key_with_a_clear_message() {
    let dir = tempfile::tempdir().unwrap();
    let global = host(401, INVALID_KEY).await;
    let mainland = host(401, INVALID_KEY).await;
    let (provider, _) = setup(&global, &mainland, &dir, false);
    let error = provider.validate_key(SECRET).await.unwrap_err();
    assert_eq!(
        error.to_string(),
        "Moonshot rejected this API key on api.moonshot.ai and api.moonshot.cn; \
         check it at https://platform.kimi.ai/console/api-keys"
    );
}

#[tokio::test]
async fn discovery_lists_stored_key_accounts() {
    let dir = tempfile::tempdir().unwrap();
    let global = host(200, BALANCE).await;
    let (provider, account) = setup(&global, &global, &dir, true);
    assert_eq!(provider.discover().await, Ok(Vec::new()));
    std::fs::create_dir_all(&account.home).unwrap();
    key_accounts::save_record(&account.home, &identity(SECRET)).unwrap();
    assert_eq!(provider.discover().await.unwrap(), [account]);
    assert_eq!(provider.usage_homes().await, Ok(Vec::new()));
}
