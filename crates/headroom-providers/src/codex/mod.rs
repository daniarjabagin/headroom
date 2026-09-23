mod auth;
mod client;
mod env;
mod jwt;
mod labels;
mod local_usage;
mod mapper;
mod number;
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
use headroom_core::account::{AccountIdentity, AccountRef, CredentialOwner, ProviderKind};
use headroom_core::cursor::LogCursors;
use headroom_core::event::UsageEvent;
use headroom_core::provider::{Provider, ProviderError};
use headroom_core::quota::LimitsSnapshot;
use jiff::Timestamp;

use auth::{Credentials, load_credentials};
use client::UsageClient;

pub use client::DEFAULT_API_BASE;
pub use env::CodexEnvironment;

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
        let client = UsageClient::new(&config.api_base)?;
        Ok(CodexProvider { config, client })
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
        Ok(mapper::map_usage(
            &response,
            credentials.identity.clone(),
            now,
        ))
    }
}

#[async_trait]
impl Provider for CodexProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Codex
    }

    async fn discover(&self) -> Result<Vec<AccountRef>, ProviderError> {
        discover_accounts(self.config.environment.homes()?)
    }

    async fn fetch_limits(&self, account: &AccountRef) -> Result<LimitsSnapshot, ProviderError> {
        let credentials = load_credentials(&account.home)?;
        let now = self.now();
        match self.fetch_live(&credentials, now).await {
            Err(error) if allows_offline_fallback(&error) => {
                offline_fallback(account.home.clone(), credentials.identity, now, error).await
            }
            result => result,
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
        id: credentials.identity.account_id(ProviderKind::Codex),
        provider: ProviderKind::Codex,
        home,
        owner,
    }
}

fn push_unique(accounts: &mut Vec<AccountRef>, account: AccountRef) {
    if accounts.iter().all(|known| known.id != account.id) {
        accounts.push(account);
    }
}

fn allows_offline_fallback(error: &ProviderError) -> bool {
    matches!(
        error,
        ProviderError::Network(_)
            | ProviderError::SignInExpired
            | ProviderError::RateLimited { .. }
    )
}

async fn offline_fallback(
    home: PathBuf,
    identity: AccountIdentity,
    now: Timestamp,
    error: ProviderError,
) -> Result<LimitsSnapshot, ProviderError> {
    let lookup =
        tokio::task::spawn_blocking(move || rate_limits::latest_snapshot(&home, identity, now))
            .await;
    match lookup {
        Ok(Ok(Some(snapshot))) => Ok(snapshot),
        Ok(Ok(None)) => Err(error),
        Ok(Err(local)) => {
            tracing::warn!(error = %local, "codex offline limits unavailable");
            Err(error)
        }
        Err(join) => {
            tracing::warn!(error = %join, "codex offline limits lookup failed");
            Err(error)
        }
    }
}

#[cfg(test)]
#[path = "provider_tests.rs"]
mod tests;
