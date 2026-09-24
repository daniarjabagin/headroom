use std::collections::HashMap;
use std::fs;

use headroom_core::account::AccountId;
use headroom_core::quota::WindowId;
use tempfile::TempDir;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::*;

const OK: &str = include_str!("fixtures/usage_ok.json");
const NO_PLAN: &str = include_str!("fixtures/no_subscription.json");
const INVALID_KEY: &str = include_str!("fixtures/invalid_key.json");
const AUTH: &str = include_str!("fixtures/auth.json");
const CLI_KEY: &str = "sk-fake-go-key";
const STORED_KEY: &str = "oc_sk_fake_stored";

#[derive(Default)]
struct Secrets(HashMap<AccountId, String>);

#[async_trait]
impl SecretReader for Secrets {
    async fn read_secret(
        &self,
        account: &AccountId,
    ) -> Result<Option<SecretString>, ProviderError> {
        Ok(self.0.get(account).cloned().map(SecretString::new))
    }
}

fn fixed_now() -> Timestamp {
    "2026-09-23T10:00:00Z".parse().unwrap()
}

struct Sandbox {
    dir: TempDir,
    config: OpenCodeConfig,
}

impl Sandbox {
    fn new(api_base: &str) -> Sandbox {
        let dir = tempfile::tempdir().unwrap();
        let config = OpenCodeConfig {
            api_base: api_base.to_owned(),
            ..OpenCodeConfig::for_home(dir.path())
        };
        Sandbox { dir, config }
    }

    fn with_cli_login(self, text: &str) -> Sandbox {
        fs::create_dir_all(&self.config.data_dir).unwrap();
        fs::write(self.config.data_dir.join("auth.json"), text).unwrap();
        self
    }

    fn add_key_account(&self, name: &str, key: &str) -> AccountRef {
        let home = self.config.accounts_dir.join(name);
        fs::create_dir_all(&home).unwrap();
        key_accounts::save_record(&home, &identity_for_key(key)).unwrap();
        AccountRef {
            id: identity_for_key(key).account_id(&ID),
            provider: ID,
            home,
            owner: CredentialOwner::Headroom,
        }
    }

    fn provider(&self, secrets: Secrets) -> OpenCodeProvider {
        let http = crate::http::client().unwrap();
        OpenCodeProvider::with_clock(self.config.clone(), http, Arc::new(secrets), fixed_now)
    }
}

fn stored(account: &AccountRef, key: &str) -> Secrets {
    Secrets(HashMap::from([(account.id.clone(), key.to_owned())]))
}

async fn usage_server(key: &str, status: u16, body: &str) -> MockServer {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/zen/go/v1/usage"))
        .and(header("authorization", format!("Bearer {key}").as_str()))
        .respond_with(ResponseTemplate::new(status).set_body_string(body))
        .expect(1)
        .mount(&server)
        .await;
    server
}

#[tokio::test]
async fn discovery_lists_pasted_keys_first_then_the_opencode_login() {
    let sandbox = Sandbox::new(DEFAULT_API_BASE).with_cli_login(AUTH);
    let pasted = sandbox.add_key_account("a", STORED_KEY);
    let same_as_cli = sandbox.add_key_account("b", CLI_KEY);
    let accounts = sandbox
        .provider(Secrets::default())
        .discover()
        .await
        .unwrap();
    assert_eq!(accounts[..2], [pasted, same_as_cli.clone()]);
    assert_eq!(accounts.len(), 3);
    assert_eq!(accounts[2].owner, CredentialOwner::Cli);
    assert_eq!(accounts[2].id, same_as_cli.id);

    let cli_only = Sandbox::new(DEFAULT_API_BASE).with_cli_login(AUTH);
    let accounts = cli_only
        .provider(Secrets::default())
        .discover()
        .await
        .unwrap();
    assert_eq!(accounts.len(), 1);
    assert_eq!(accounts[0].owner, CredentialOwner::Cli);
    assert_eq!(accounts[0].home, cli_only.config.data_dir);
    assert_eq!(accounts[0].id, identity_for_key(CLI_KEY).account_id(&ID));
    assert!(sandbox.dir.path().exists());
}

#[tokio::test]
async fn a_broken_login_fails_discovery_only_when_nothing_else_is_found() {
    let broken = Sandbox::new(DEFAULT_API_BASE).with_cli_login("{");
    let provider = broken.provider(Secrets::default());
    assert!(matches!(
        provider.discover().await,
        Err(ProviderError::LocalData(_))
    ));
    let pasted = broken.add_key_account("a", STORED_KEY);
    assert_eq!(provider.discover().await, Ok(vec![pasted]));
    let nothing = Sandbox::new(DEFAULT_API_BASE);
    let provider = nothing.provider(Secrets::default());
    assert_eq!(provider.discover().await, Ok(Vec::new()));
    assert_eq!(provider.usage_homes().await, Ok(Vec::new()));
}

#[tokio::test]
async fn limits_use_the_stored_key_of_a_pasted_account() {
    let server = usage_server(STORED_KEY, 200, OK).await;
    let sandbox = Sandbox::new(&server.uri());
    let account = sandbox.add_key_account("a", STORED_KEY);
    let provider = sandbox.provider(stored(&account, STORED_KEY));
    let snapshot = provider.fetch_limits(&account).await.unwrap();
    let ids: Vec<_> = snapshot.windows.iter().map(|w| w.id.clone()).collect();
    assert_eq!(
        ids,
        [
            WindowId::Session,
            WindowId::Weekly,
            WindowId::Other("monthly".into())
        ]
    );
    assert_eq!(snapshot.identity, identity_for_key(STORED_KEY));
    assert_eq!(snapshot.fetched_at, fixed_now());
    assert_eq!(snapshot.source, LimitsSource::Live);
    assert!(snapshot.balances.is_empty());
}

#[tokio::test]
async fn limits_use_the_opencode_login_read_only() {
    let server = usage_server(CLI_KEY, 200, OK).await;
    let sandbox = Sandbox::new(&server.uri()).with_cli_login(AUTH);
    let provider = sandbox.provider(Secrets::default());
    let account = provider.discover().await.unwrap().remove(0);
    let snapshot = provider.fetch_limits(&account).await.unwrap();
    assert_eq!(snapshot.windows.len(), 3);
    let auth = fs::read_to_string(sandbox.config.data_dir.join("auth.json")).unwrap();
    assert_eq!(auth, AUTH);
}

#[tokio::test]
async fn a_lapsed_plan_is_reported_as_no_subscription() {
    let server = usage_server(STORED_KEY, 403, NO_PLAN).await;
    let sandbox = Sandbox::new(&server.uri());
    let account = sandbox.add_key_account("a", STORED_KEY);
    let error = sandbox
        .provider(stored(&account, STORED_KEY))
        .fetch_limits(&account)
        .await
        .unwrap_err();
    assert!(matches!(error, ProviderError::NoSubscription { .. }));
}

#[tokio::test]
async fn a_missing_or_replaced_key_never_calls_the_endpoint() {
    let sandbox = Sandbox::new("http://127.0.0.1:9");
    let account = sandbox.add_key_account("a", STORED_KEY);
    let provider = sandbox.provider(Secrets::default());
    assert_eq!(
        provider.fetch_limits(&account).await,
        Err(ProviderError::NotSignedIn)
    );
    let replaced = sandbox.provider(stored(&account, "oc_sk_other"));
    assert!(matches!(
        replaced.fetch_limits(&account).await,
        Err(ProviderError::LocalData(ref m)) if m.contains("has changed")
    ));
}

#[tokio::test]
async fn validation_accepts_a_subscribed_key_and_names_rejections() {
    let server = usage_server(STORED_KEY, 200, OK).await;
    let provider = Sandbox::new(&server.uri()).provider(Secrets::default());
    assert_eq!(
        provider.validate_key(STORED_KEY).await,
        Ok(identity_for_key(STORED_KEY))
    );
    let server = usage_server(STORED_KEY, 401, INVALID_KEY).await;
    let provider = Sandbox::new(&server.uri()).provider(Secrets::default());
    assert_eq!(
        provider.validate_key(STORED_KEY).await,
        Err(ProviderError::Unsupported(
            "OpenCode rejected this API key; check it at https://opencode.ai/auth".into()
        ))
    );
    let server = usage_server(STORED_KEY, 403, NO_PLAN).await;
    let provider = Sandbox::new(&server.uri()).provider(Secrets::default());
    assert!(matches!(
        provider.validate_key(STORED_KEY).await,
        Err(ProviderError::NoSubscription { .. })
    ));
}

#[test]
fn the_descriptor_prefers_a_pasted_key() {
    assert_eq!(DESCRIPTOR.validate(), Ok(()));
    assert!(matches!(
        DESCRIPTOR.default_method(),
        Some(AddAccountMethod::ApiKey(prompt)) if prompt.console_url == "https://opencode.ai/auth"
    ));
    assert!(DESCRIPTOR.accepts_api_key());
}
