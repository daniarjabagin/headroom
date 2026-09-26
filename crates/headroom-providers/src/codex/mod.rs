mod auth;
mod client;
mod env;
mod jwt;
mod keyring;
mod labels;
mod local_usage;
mod mapper;
mod number;
mod oauth;
mod offline;
mod plan;
mod rate_limits;
mod refresh;
mod reverse;
#[cfg(test)]
mod test_support;
mod timestamp;
mod usage_parser;
mod windows;

use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use headroom_core::account::{AccountRef, CredentialOwner, ProviderId};
use headroom_core::cursor::LogCursors;
use headroom_core::descriptor::{
    AddAccountMethod, CliLogin, HomeVar, ProviderDescriptor, ProviderLinks,
};
use headroom_core::event::UsageEvent;
use headroom_core::provider::{Provider, ProviderError};
use headroom_core::quota::LimitsSnapshot;
use jiff::Timestamp;

use auth::Credentials;
use client::UsageClient;
use oauth::TokenClient;

use crate::http;
use crate::keychain::Security;

pub use client::DEFAULT_API_BASE;
pub use env::CodexEnvironment;
pub use oauth::DEFAULT_AUTH_BASE;

pub const ID: ProviderId = ProviderId::from_static("codex");

pub static DESCRIPTOR: ProviderDescriptor = ProviderDescriptor {
    id: ID,
    display_name: "Codex",
    add_account: &[AddAccountMethod::CliLogin(CliLogin {
        program: "codex",
        args: &["login"],
        home_var: HomeVar::Direct("CODEX_HOME"),
        credentials_file: "auth.json",
        default_dir: ".codex",
        needs_pty: false,
        scrub_env: &[],
    })],
    multi_account: true,
    local_usage: true,
    min_poll_interval: None,
    links: ProviderLinks {
        status: Some("https://status.openai.com"),
        dashboard: Some("https://chatgpt.com/codex"),
        usage: None,
    },
};

pub type Clock = Arc<dyn Fn() -> Timestamp + Send + Sync>;

#[derive(Clone)]
pub struct CodexConfig {
    pub environment: CodexEnvironment,
    pub api_base: String,
    pub auth_base: String,
    pub clock: Clock,
}

impl CodexConfig {
    #[must_use]
    pub fn from_process() -> CodexConfig {
        CodexConfig {
            environment: CodexEnvironment::from_process(),
            api_base: DEFAULT_API_BASE.to_owned(),
            auth_base: DEFAULT_AUTH_BASE.to_owned(),
            clock: Arc::new(Timestamp::now),
        }
    }
}

impl fmt::Debug for CodexConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CodexConfig")
            .field("environment", &self.environment)
            .field("api_base", &self.api_base)
            .field("auth_base", &self.auth_base)
            .finish_non_exhaustive()
    }
}

#[derive(Debug)]
pub struct CodexProvider {
    config: CodexConfig,
    client: UsageClient,
    tokens: TokenClient,
}

impl CodexProvider {
    pub fn new(config: CodexConfig) -> Result<CodexProvider, ProviderError> {
        Ok(CodexProvider::with_http(config, http::client()?))
    }

    #[must_use]
    pub fn with_http(config: CodexConfig, http: reqwest::Client) -> CodexProvider {
        let client = UsageClient::new(http.clone(), &config.api_base);
        let tokens = TokenClient::new(http, &config.auth_base);
        CodexProvider {
            config,
            client,
            tokens,
        }
    }

    fn now(&self) -> Timestamp {
        (self.config.clock)()
    }

    async fn current_credentials(
        &self,
        account: &AccountRef,
    ) -> Result<Credentials, ProviderError> {
        let keychain = self.config.environment.keychain.as_ref();
        let credentials = keyring::load(keychain, &account.home).await?;
        ensure_same_account(account, credentials)
    }

    fn refreshable(&self, account: &AccountRef) -> bool {
        let keychain = self.config.environment.keychain.as_ref();
        account.owner == CredentialOwner::Headroom
            && keyring::reads_file(keychain, &account.home).unwrap_or(false)
    }

    async fn fetch_refreshing(
        &self,
        account: &AccountRef,
        credentials: Credentials,
        now: Timestamp,
    ) -> (Credentials, Result<LimitsSnapshot, ProviderError>) {
        match self.fetch_live(&credentials, now).await {
            Err(ProviderError::SignInExpired) if self.refreshable(account) => {
                let refreshed = refresh::refresh(&self.tokens, &account.home, &credentials, now)
                    .await
                    .and_then(|fresh| ensure_same_account(account, fresh));
                match refreshed {
                    Ok(fresh) => {
                        let live = self.fetch_live(&fresh, now).await;
                        (fresh, live)
                    }
                    Err(error) => (credentials, Err(error)),
                }
            }
            live => (credentials, live),
        }
    }

    async fn fetch_live(
        &self,
        credentials: &Credentials,
        now: Timestamp,
    ) -> Result<LimitsSnapshot, ProviderError> {
        credentials.ensure_fresh(now)?;
        let response = self.client.fetch_usage(credentials, now).await?;
        let snapshot = mapper::map_usage(&response, credentials.identity.clone(), now);
        plan::require_subscription(response.plan_type.as_deref(), snapshot)
    }
}

#[async_trait]
impl Provider for CodexProvider {
    fn descriptor(&self) -> &'static ProviderDescriptor {
        &DESCRIPTOR
    }

    async fn discover(&self) -> Result<Vec<AccountRef>, ProviderError> {
        let environment = &self.config.environment;
        discover_accounts(environment.keychain.as_ref(), environment.homes()?).await
    }

    async fn account_at(&self, home: &Path) -> Result<Option<AccountRef>, ProviderError> {
        match keyring::load(self.config.environment.keychain.as_ref(), home).await {
            Ok(credentials) => Ok(Some(account_ref(
                home.to_path_buf(),
                CredentialOwner::Headroom,
                &credentials,
            ))),
            Err(ProviderError::NotSignedIn) => Ok(None),
            Err(error) => Err(error),
        }
    }

    async fn usage_homes(&self) -> Result<Vec<PathBuf>, ProviderError> {
        self.config.environment.usage_homes()
    }

    async fn fetch_limits(&self, account: &AccountRef) -> Result<LimitsSnapshot, ProviderError> {
        let credentials = self.current_credentials(account).await?;
        let now = self.now();
        match self.fetch_refreshing(account, credentials, now).await {
            (credentials, Err(error)) => {
                offline::limits_from_logs(account.home.clone(), credentials, now, error).await
            }
            (_, live) => live,
        }
    }

    fn read_usage(
        &self,
        home: &Path,
        cursors: &mut LogCursors,
    ) -> Result<Vec<UsageEvent>, ProviderError> {
        local_usage::read_usage(home, cursors, self.now())
    }
}

async fn discover_accounts(
    keychain: Option<&Security>,
    homes: Vec<(PathBuf, CredentialOwner)>,
) -> Result<Vec<AccountRef>, ProviderError> {
    let mut accounts: Vec<AccountRef> = Vec::new();
    let mut cli_error = None;
    for (home, owner) in homes {
        match keyring::load(keychain, &home).await {
            Ok(credentials) => accounts.push(account_ref(home, owner, &credentials)),
            Err(error) if owner == CredentialOwner::Cli => cli_error = Some(error),
            Err(error) => tracing::warn!(home = %home.display(), %error, "skipping codex home"),
        }
    }
    if accounts.is_empty() {
        Err(cli_error.unwrap_or(ProviderError::NotSignedIn))
    } else {
        Ok(accounts)
    }
}

fn ensure_same_account(
    account: &AccountRef,
    credentials: Credentials,
) -> Result<Credentials, ProviderError> {
    if credentials.identity.account_id(&ID) == account.id {
        Ok(credentials)
    } else {
        Err(ProviderError::AccountChanged(format!(
            "the Codex account signed in at {} has changed",
            account.home.display()
        )))
    }
}

fn account_ref(home: PathBuf, owner: CredentialOwner, credentials: &Credentials) -> AccountRef {
    AccountRef {
        id: credentials.identity.account_id(&ID),
        provider: ID,
        home,
        owner,
    }
}

#[cfg(test)]
#[path = "provider_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "fallback_tests.rs"]
mod fallback_tests;

#[cfg(test)]
#[path = "owned_refresh_tests.rs"]
mod owned_refresh_tests;
