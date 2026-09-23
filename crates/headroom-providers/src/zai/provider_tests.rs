use std::collections::HashMap;

use headroom_core::account::{AccountId, CredentialOwner};
use headroom_core::quota::WindowId;
use headroom_core::secret::SecretString;
use tempfile::TempDir;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::*;

const QUOTA: &str = include_str!("fixtures/quota_credits.json");
const NO_PLAN: &str = include_str!("fixtures/no_plan.json");
const INVALID_KEY: &str = include_str!("fixtures/invalid_key.json");
const SUBSCRIPTION: &str = include_str!("fixtures/subscription.json");
const SECRET: &str = "zk-test";

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

async fn server(quota: (u16, &str), subscription: (u16, &str)) -> MockServer {
    let server = MockServer::start().await;
    let routes = [
        ("/api/monitor/usage/quota/limit", quota),
        ("/api/biz/subscription/list", subscription),
    ];
    for (route, (status, body)) in routes {
        Mock::given(method("GET"))
            .and(path(route))
            .respond_with(ResponseTemplate::new(status).set_body_string(body))
            .mount(&server)
            .await;
    }
    server
}

fn setup(server: &MockServer, dir: &TempDir, stored: bool) -> (ZaiProvider, AccountRef) {
    let account = AccountRef {
        id: identity(SECRET, None).account_id(&ID),
        provider: ID,
        home: dir.path().join("accounts/zai/a"),
        owner: CredentialOwner::Headroom,
    };
    let keys = if stored {
        HashMap::from([(account.id.clone(), SecretString::new(SECRET.into()))])
    } else {
        HashMap::new()
    };
    let config = ZaiConfig {
        accounts_dir: dir.path().join("accounts/zai"),
        api_base: server.uri(),
    };
    let provider = ZaiProvider::new(config, crate::http::client().unwrap(), Arc::new(Keys(keys)))
        .with_clock(fixed_now);
    (provider, account)
}

#[tokio::test]
async fn a_plan_account_reports_its_windows_and_plan_name() {
    let dir = tempfile::tempdir().unwrap();
    let server = server((200, QUOTA), (200, SUBSCRIPTION)).await;
    let (provider, account) = setup(&server, &dir, true);
    let snapshot = provider.fetch_limits(&account).await.unwrap();
    assert_eq!(snapshot.identity.plan.as_deref(), Some("GLM Coding Pro"));
    assert_eq!(snapshot.identity.account_id(&ID), account.id);
    let ids: Vec<_> = snapshot.windows.iter().map(|w| w.id.clone()).collect();
    assert_eq!(ids, [WindowId::Session, WindowId::Weekly]);
    assert_eq!(snapshot.balances, []);
    assert_eq!(snapshot.fetched_at, fixed_now());
}

#[tokio::test]
async fn a_failed_plan_lookup_keeps_the_windows() {
    let dir = tempfile::tempdir().unwrap();
    let server = server((200, QUOTA), (500, "")).await;
    let (provider, account) = setup(&server, &dir, true);
    let snapshot = provider.fetch_limits(&account).await.unwrap();
    assert_eq!(snapshot.identity.plan, None);
    assert_eq!(snapshot.windows.len(), 2);
}

#[tokio::test]
async fn no_plan_is_a_missing_subscription_but_the_key_is_accepted() {
    let dir = tempfile::tempdir().unwrap();
    let server = server((200, NO_PLAN), (200, SUBSCRIPTION)).await;
    let (provider, account) = setup(&server, &dir, true);
    assert!(matches!(
        provider.fetch_limits(&account).await,
        Err(ProviderError::NoSubscription { .. })
    ));
    let identity = provider.validate_key(SECRET).await.unwrap();
    assert_eq!(identity.account_id(&ID), account.id);
    assert_eq!(identity.plan, None);
}

#[tokio::test]
async fn an_invalid_key_is_signed_out_and_rejected_on_validation() {
    let dir = tempfile::tempdir().unwrap();
    let server = server((401, INVALID_KEY), (401, INVALID_KEY)).await;
    let (provider, account) = setup(&server, &dir, true);
    assert_eq!(
        provider.fetch_limits(&account).await,
        Err(ProviderError::SignInExpired)
    );
    let error = provider.validate_key(SECRET).await.unwrap_err();
    assert_eq!(
        error.to_string(),
        "Z.ai rejected this API key; check it at https://z.ai/manage-apikey/apikey-list"
    );
}

#[tokio::test]
async fn validation_names_the_key_by_its_hash_and_plan() {
    let dir = tempfile::tempdir().unwrap();
    let server = server((200, QUOTA), (200, SUBSCRIPTION)).await;
    let (provider, account) = setup(&server, &dir, false);
    let identity = provider.validate_key(SECRET).await.unwrap();
    assert_eq!(identity.account_id(&ID), account.id);
    assert_eq!(identity.plan.as_deref(), Some("GLM Coding Pro"));
    assert!(!identity.stable_key.contains(SECRET));
    assert_eq!(
        provider.fetch_limits(&account).await,
        Err(ProviderError::NotSignedIn)
    );
}

#[tokio::test]
async fn discovery_lists_stored_key_accounts() {
    let dir = tempfile::tempdir().unwrap();
    let server = server((200, QUOTA), (200, SUBSCRIPTION)).await;
    let (provider, account) = setup(&server, &dir, true);
    assert_eq!(provider.discover().await, Ok(Vec::new()));
    std::fs::create_dir_all(&account.home).unwrap();
    key_accounts::save_record(&account.home, &identity(SECRET, None)).unwrap();
    assert_eq!(provider.discover().await.unwrap(), [account]);
    assert_eq!(provider.usage_homes().await, Ok(Vec::new()));
}
