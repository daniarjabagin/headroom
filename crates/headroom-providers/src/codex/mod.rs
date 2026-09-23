mod auth;
mod client;
mod env;
mod jwt;
mod labels;
mod local_usage;
mod mapper;
mod number;
mod offline;
mod plan;
mod rate_limits;
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
use headroom_core::descriptor::{AddAccountMethod, CliLogin, HomeVar, ProviderDescriptor};
use headroom_core::event::UsageEvent;
use headroom_core::provider::{Provider, ProviderError};
use headroom_core::quota::LimitsSnapshot;
use jiff::Timestamp;

use auth::{Credentials, load_credentials};
use client::UsageClient;

use crate::http;

pub use client::DEFAULT_API_BASE;
pub use env::CodexEnvironment;

pub const ID: ProviderId = ProviderId::from_static("codex");

pub static DESCRIPTOR: ProviderDescriptor = ProviderDescriptor {
    id: ID,
    display_name: "Codex",
    add_account: &[AddAccountMethod::CliLogin(CliLogin {
        program: "codex",
        args: &["login"],
        home_var: HomeVar::Direct("CODEX_HOME"),
        credentials_file: "auth.json",
        needs_pty: false,
    })],
    multi_account: true,
    local_usage: true,
};

pub type Clock = Arc<dyn Fn() -> Timestamp + Send + Sync>;

#[derive(Clone)]
pub struct CodexConfig {
    pub environment: CodexEnvironment,
    pub api_base: String,
    pub clock: Clock,
}

impl CodexConfig {
    #[must_use]
    pub fn from_process() -> CodexConfig {
        CodexConfig {
            environment: CodexEnvironment::from_process(),
            api_base: DEFAULT_API_BASE.to_owned(),
            clock: Arc::new(Timestamp::now),
        }
    }
}

impl fmt::Debug for CodexConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CodexConfig")
            .field("environment", &self.environment)
            .field("api_base", &self.api_base)
            .finish_non_exhaustive()
    }
}

#[derive(Debug)]
pub struct CodexProvider {
    config: CodexConfig,
    client: UsageClient,
}

impl CodexProvider {
    pub fn new(config: CodexConfig) -> Result<CodexProvider, ProviderError> {
        Ok(CodexProvider::with_http(config, http::client()?))
    }

    #[must_use]
    pub fn with_http(config: CodexConfig, http: reqwest::Client) -> CodexProvider {
        let client = UsageClient::new(http, &config.api_base);
        CodexProvider { config, client }
    }

    fn now(&self) -> Timestamp {
        (self.config.clock)()
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
        discover_accounts(self.config.environment.homes()?)
    }

    async fn account_at(&self, home: &Path) -> Result<Option<AccountRef>, ProviderError> {
        match load_credentials(home) {
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
        let credentials = current_credentials(account)?;
        let now = self.now();
        match self.fetch_live(&credentials, now).await {
            Err(error) => {
                offline::limits_from_logs(account.home.clone(), credentials, now, error).await
            }
            live => live,
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

fn current_credentials(account: &AccountRef) -> Result<Credentials, ProviderError> {
    let credentials = load_credentials(&account.home)?;
    if credentials.identity.account_id(&ID) == account.id {
        Ok(credentials)
    } else {
        Err(ProviderError::LocalData(format!(
            "the Codex account signed in at {} has changed",
            account.home.display()
        )))
    }
}

fn discover_accounts(
    homes: Vec<(PathBuf, CredentialOwner)>,
) -> Result<Vec<AccountRef>, ProviderError> {
    let mut accounts: Vec<AccountRef> = Vec::new();
    let mut cli_error = None;
    for (home, owner) in homes {
        match load_credentials(&home) {
            Ok(credentials) => push_unique(&mut accounts, account_ref(home, owner, &credentials)),
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

fn account_ref(home: PathBuf, owner: CredentialOwner, credentials: &Credentials) -> AccountRef {
    AccountRef {
        id: credentials.identity.account_id(&ID),
        provider: ID,
        home,
        owner,
    }
}

fn push_unique(accounts: &mut Vec<AccountRef>, account: AccountRef) {
    if accounts.iter().all(|known| known.id != account.id) {
        accounts.push(account);
    }
}

#[cfg(test)]
#[path = "provider_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "fallback_tests.rs"]
mod fallback_tests;
