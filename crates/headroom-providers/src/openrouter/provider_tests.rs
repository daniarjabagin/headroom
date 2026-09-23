use std::collections::HashMap;

use headroom_core::account::{AccountId, CredentialOwner};
use headroom_core::quota::{BalanceAmount, WindowId};
use headroom_core::secret::SecretString;
use headroom_core::units::MicroUsd;
use tempfile::TempDir;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::*;

const KEY: &str = include_str!("fixtures/key.json");
const KEY_LIMITED: &str = include_str!("fixtures/key_limited.json");
const CREDITS: &str = include_str!("fixtures/credits.json");
const CREDITS_FORBIDDEN: &str = include_str!("fixtures/credits_forbidden.json");
const INVALID_KEY: &str = include_str!("fixtures/invalid_key.json");
const SECRET: &str = "sk-or-v1-test";

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
    "2026-09-23T10:00:00Z".parse().unwrap()
}

async fn server(key: (u16, &str), credits: (u16, &str)) -> MockServer {
    let server = MockServer::start().await;
    for (endpoint, (status, body)) in [("/api/v1/key", key), ("/api/v1/credits", credits)] {
        Mock::given(method("GET"))
            .and(path(endpoint))
            .and(header("authorization", format!("Bearer {SECRET}")))
            .respond_with(ResponseTemplate::new(status).set_body_string(body))
            .mount(&server)
            .await;
    }
    server
}

fn setup(server: &MockServer, dir: &TempDir, stored: bool) -> (OpenRouterProvider, AccountRef) {
    let account = AccountRef {
        id: identity(SECRET, None).account_id(&ID),
        provider: ID,
        home: dir.path().join("accounts/openrouter/a"),
        owner: CredentialOwner::Headroom,
    };
    let keys = if stored {
        HashMap::from([(account.id.clone(), SecretString::new(SECRET.into()))])
    } else {
        HashMap::new()
    };
    let config = OpenRouterConfig {
        accounts_dir: dir.path().join("accounts/openrouter"),
        api_base: server.uri(),
    };
    let provider =
        OpenRouterProvider::new(config, crate::http::client().unwrap(), Arc::new(Keys(keys)))
            .with_clock(fixed_now);
    (provider, account)
}

#[tokio::test]
async fn a_regular_key_reports_spend_limit_and_the_management_key_notice() {
    let dir = tempfile::tempdir().unwrap();
    let server = server((200, KEY_LIMITED), (403, CREDITS_FORBIDDEN)).await;
    let (provider, account) = setup(&server, &dir, true);
    let snapshot = provider.fetch_limits(&account).await.unwrap();
    assert_eq!(snapshot.identity.plan.as_deref(), Some("Free tier"));
    assert_eq!(snapshot.identity.account_id(&ID), account.id);
    assert_eq!(snapshot.fetched_at, fixed_now());
    assert_eq!(snapshot.source, LimitsSource::Live);
    assert_eq!(snapshot.windows[0].id, WindowId::Other("key_limit".into()));
    assert_eq!(
        snapshot.notices[0].text,
        "Credit balance needs a management key"
    );
    let ids: Vec<_> = snapshot.balances.iter().map(|b| b.id.as_str()).collect();
    assert_eq!(ids, ["spend_today", "spend_week", "spend_month"]);
}

#[tokio::test]
async fn a_management_key_reports_the_credit_balance() {
    let dir = tempfile::tempdir().unwrap();
    let server = server((200, KEY), (200, CREDITS)).await;
    let (provider, account) = setup(&server, &dir, true);
    let snapshot = provider.fetch_limits(&account).await.unwrap();
    assert_eq!(snapshot.notices, []);
    assert_eq!(
        snapshot.balances[0].amount,
        BalanceAmount::Usd(MicroUsd(108_123_579))
    );
}

#[tokio::test]
async fn a_revoked_key_is_signed_out_and_a_missing_key_is_not_signed_in() {
    let dir = tempfile::tempdir().unwrap();
    let server = server((401, INVALID_KEY), (401, INVALID_KEY)).await;
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
async fn validation_names_the_key_owner_by_its_hash() {
    let dir = tempfile::tempdir().unwrap();
    let server = server((200, KEY), (403, CREDITS_FORBIDDEN)).await;
    let (provider, account) = setup(&server, &dir, false);
    let identity = provider.validate_key(SECRET).await.unwrap();
    assert_eq!(identity.account_id(&ID), account.id);
    assert_eq!(identity.plan.as_deref(), Some("Pay as you go"));
    assert!(identity.stable_key.starts_with("key-sha256:"));
    assert!(!identity.stable_key.contains(SECRET));
}

#[tokio::test]
async fn validation_rejects_an_invalid_key_with_a_clear_message() {
    let dir = tempfile::tempdir().unwrap();
    let server = server((401, INVALID_KEY), (401, INVALID_KEY)).await;
    let (provider, _) = setup(&server, &dir, false);
    let error = provider.validate_key(SECRET).await.unwrap_err();
    assert_eq!(
        error.to_string(),
        "OpenRouter rejected this API key; check it at https://openrouter.ai/settings/keys"
    );
}

#[tokio::test]
async fn discovery_lists_stored_key_accounts() {
    let dir = tempfile::tempdir().unwrap();
    let server = server((200, KEY), (200, CREDITS)).await;
    let (provider, account) = setup(&server, &dir, true);
    assert_eq!(provider.discover().await, Ok(Vec::new()));
    std::fs::create_dir_all(&account.home).unwrap();
    key_accounts::save_record(&account.home, &identity(SECRET, None)).unwrap();
    let found = provider.discover().await.unwrap();
    assert_eq!(found, [account]);
    assert_eq!(provider.usage_homes().await, Ok(Vec::new()));
}
