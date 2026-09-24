use std::collections::HashMap;
use std::fs;

use headroom_core::account::{AccountId, CredentialOwner};
use headroom_core::pace::Tone;
use headroom_core::quota::BalanceAmount;
use headroom_core::units::MicroUsd;
use tempfile::TempDir;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::*;

const BALANCE: &str = include_str!("fixtures/balance.json");
const DEPLETED: &str = include_str!("fixtures/balance_depleted.json");
const PROFILE: &str = include_str!("fixtures/profile.json");
const AUTH_ORG: &str = include_str!("fixtures/auth_organization.json");
const KEY: &str = "kilo-pasted-key";
const CLI_TOKEN: &str = "kilo-fake-org-token";

#[derive(Default)]
struct Keys(HashMap<AccountId, String>);

#[async_trait]
impl SecretReader for Keys {
    async fn read_secret(
        &self,
        account: &AccountId,
    ) -> Result<Option<SecretString>, ProviderError> {
        Ok(self.0.get(account).cloned().map(SecretString::new))
    }
}

fn fixed_now() -> Timestamp {
    "2026-09-24T10:00:00Z".parse().unwrap()
}

async fn mount(server: &MockServer, endpoint: &str, token: &str, status: u16, body: &str) {
    Mock::given(method("GET"))
        .and(path(endpoint))
        .and(header("authorization", format!("Bearer {token}")))
        .respond_with(ResponseTemplate::new(status).set_body_string(body))
        .mount(server)
        .await;
}

struct Sandbox {
    _dir: TempDir,
    config: KiloConfig,
}

impl Sandbox {
    fn new(server: &MockServer) -> Sandbox {
        let dir = tempfile::tempdir().unwrap();
        let config = KiloConfig {
            api_base: server.uri(),
            ..KiloConfig::for_home(dir.path())
        };
        Sandbox { _dir: dir, config }
    }

    fn provider(&self, keys: Keys) -> KiloProvider {
        let http = crate::http::client().unwrap();
        KiloProvider::with_clock(self.config.clone(), http, Arc::new(keys), fixed_now)
    }

    fn add_key_account(&self) -> (AccountRef, Keys) {
        let home = self.config.accounts_dir.join("a");
        fs::create_dir_all(&home).unwrap();
        let identity = key_identity();
        key_accounts::save_record(&home, &identity).unwrap();
        let id = identity.account_id(&ID);
        let account = AccountRef {
            id: id.clone(),
            provider: ID,
            home,
            owner: CredentialOwner::Headroom,
        };
        (account, Keys(HashMap::from([(id, KEY.to_owned())])))
    }
}

fn key_identity() -> AccountIdentity {
    KiloToken {
        token: SecretString::new(KEY.to_owned()),
        organization_id: None,
    }
    .identity()
}

#[tokio::test]
async fn a_key_account_shows_its_balance_and_profile_email() {
    let server = MockServer::start().await;
    mount(&server, "/api/profile/balance", KEY, 200, BALANCE).await;
    mount(&server, "/api/profile", KEY, 200, PROFILE).await;
    let sandbox = Sandbox::new(&server);
    let (account, keys) = sandbox.add_key_account();
    let provider = sandbox.provider(keys);
    assert_eq!(provider.discover().await, Ok(vec![account.clone()]));
    let snapshot = provider.fetch_limits(&account).await.unwrap();
    assert_eq!(snapshot.identity.email.as_deref(), Some("user@example.com"));
    assert_eq!(snapshot.identity.account_id(&ID), account.id);
    assert_eq!(snapshot.fetched_at, fixed_now());
    assert_eq!(snapshot.source, LimitsSource::Live);
    assert_eq!(
        snapshot.balances[0].amount,
        BalanceAmount::Usd(MicroUsd(18_123_456))
    );
    assert_eq!(snapshot.notices, []);
}

#[tokio::test]
async fn a_cli_organization_login_reads_the_organization_balance_without_a_profile() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/profile/balance"))
        .and(header("authorization", format!("Bearer {CLI_TOKEN}")))
        .and(header("x-kilocode-organizationid", "org-0000-fake"))
        .respond_with(ResponseTemplate::new(200).set_body_string(DEPLETED))
        .mount(&server)
        .await;
    mount(&server, "/api/profile", CLI_TOKEN, 500, "").await;
    let sandbox = Sandbox::new(&server);
    fs::create_dir_all(&sandbox.config.data_dir).unwrap();
    fs::write(sandbox.config.data_dir.join("auth.json"), AUTH_ORG).unwrap();
    let provider = sandbox.provider(Keys::default());
    let accounts = provider.discover().await.unwrap();
    assert_eq!(accounts[0].owner, CredentialOwner::Cli);
    let snapshot = provider.fetch_limits(&accounts[0]).await.unwrap();
    assert_eq!(snapshot.identity.email, None);
    assert_eq!(snapshot.balances[0].label, "Organization credits");
    assert_eq!(snapshot.notices[0].tone, Tone::Critical);
}

#[tokio::test]
async fn a_revoked_token_is_signed_out_and_a_missing_key_is_not_signed_in() {
    let server = MockServer::start().await;
    mount(&server, "/api/profile/balance", KEY, 401, "").await;
    mount(&server, "/api/profile", KEY, 401, "").await;
    let sandbox = Sandbox::new(&server);
    let (account, keys) = sandbox.add_key_account();
    assert_eq!(
        sandbox.provider(keys).fetch_limits(&account).await,
        Err(ProviderError::SignInExpired)
    );
    assert_eq!(
        sandbox
            .provider(Keys::default())
            .fetch_limits(&account)
            .await,
        Err(ProviderError::NotSignedIn)
    );
}

#[tokio::test]
async fn validation_names_the_key_by_its_hash_or_rejects_it_clearly() {
    let server = MockServer::start().await;
    mount(&server, "/api/profile/balance", KEY, 200, BALANCE).await;
    mount(&server, "/api/profile/balance", "bad-key", 401, "").await;
    let sandbox = Sandbox::new(&server);
    let provider = sandbox.provider(Keys::default());
    let identity = provider.validate_key(KEY).await.unwrap();
    assert_eq!(identity, key_identity());
    assert!(!identity.stable_key.contains(KEY));
    assert_eq!(
        provider
            .validate_key("bad-key")
            .await
            .unwrap_err()
            .to_string(),
        "Kilo rejected this API key; check it at https://app.kilo.ai/profile"
    );
}

#[test]
fn the_cli_login_writes_where_discovery_looks() {
    let AddAccountMethod::CliLogin(login) = &DESCRIPTOR.add_account[0] else {
        panic!("the CLI login comes first");
    };
    let home = Path::new("/accounts/kilo/x");
    assert_eq!(
        login.credentials_path(home),
        home.join("kilo").join("auth.json")
    );
    assert_eq!(DESCRIPTOR.validate(), Ok(()));
}
