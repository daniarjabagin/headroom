use std::collections::HashMap;

use headroom_core::account::{AccountId, CredentialOwner};
use headroom_core::quota::{BalanceAmount, WindowId};
use headroom_core::secret::SecretString;
use tempfile::TempDir;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::*;

const LIMITS: &str = include_str!("fixtures/request_limits.json");
const SECRET: &str = "wk-test-key";

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
    Mock::given(method("POST"))
        .and(path("/graphql/v2"))
        .and(header("authorization", format!("Bearer {SECRET}")))
        .respond_with(ResponseTemplate::new(status).set_body_string(body))
        .mount(&server)
        .await;
    server
}

fn setup(server: &MockServer, dir: &TempDir, stored: bool) -> (WarpProvider, AccountRef) {
    let account = AccountRef {
        id: identity(SECRET).account_id(&ID),
        provider: ID,
        home: dir.path().join("accounts/warp/a"),
        owner: CredentialOwner::Headroom,
    };
    let keys = if stored {
        HashMap::from([(account.id.clone(), SecretString::new(SECRET.into()))])
    } else {
        HashMap::new()
    };
    let config = WarpConfig {
        accounts_dir: dir.path().join("accounts/warp"),
        api_base: server.uri(),
    };
    let provider = WarpProvider::new(config, crate::http::client().unwrap(), Arc::new(Keys(keys)))
        .with_clock(fixed_now);
    (provider, account)
}

#[tokio::test]
async fn a_key_reports_monthly_credits_and_bonus_credits() {
    let dir = tempfile::tempdir().unwrap();
    let server = server(200, LIMITS).await;
    let (provider, account) = setup(&server, &dir, true);
    let snapshot = provider.fetch_limits(&account).await.unwrap();
    assert_eq!(snapshot.identity.account_id(&ID), account.id);
    assert_eq!(snapshot.fetched_at, fixed_now());
    assert_eq!(snapshot.source, LimitsSource::Live);
    assert_eq!(snapshot.windows[0].id, WindowId::Other("monthly".into()));
    assert_eq!(
        snapshot.balances[0].amount,
        BalanceAmount::Count {
            value: 1320,
            unit: "credits".into()
        }
    );
}

#[tokio::test]
async fn a_revoked_key_is_signed_out_and_a_missing_key_is_not_signed_in() {
    let dir = tempfile::tempdir().unwrap();
    let server = server(401, "").await;
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
    let server_ok = server(200, LIMITS).await;
    let (provider, account) = setup(&server_ok, &dir, false);
    let identity = provider.validate_key(SECRET).await.unwrap();
    assert_eq!(identity.account_id(&ID), account.id);
    assert!(!identity.stable_key.contains(SECRET));
    let server_bad = server(403, "").await;
    let (provider, _) = setup(&server_bad, &dir, false);
    assert_eq!(
        provider.validate_key(SECRET).await.unwrap_err().to_string(),
        "Warp rejected this API key; check it at https://docs.warp.dev/reference/cli/api-keys"
    );
}

#[tokio::test]
async fn discovery_lists_stored_key_accounts() {
    let dir = tempfile::tempdir().unwrap();
    let server = server(200, LIMITS).await;
    let (provider, account) = setup(&server, &dir, true);
    assert_eq!(provider.discover().await, Ok(Vec::new()));
    std::fs::create_dir_all(&account.home).unwrap();
    key_accounts::save_record(&account.home, &identity(SECRET)).unwrap();
    assert_eq!(provider.discover().await, Ok(vec![account]));
}
