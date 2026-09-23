mod accounts;
mod auth;
mod client;
mod config;
mod identity;
mod local_usage;
mod log_record;
mod mapper;
mod number;
mod raw;

use std::path::Path;

use async_trait::async_trait;
use headroom_core::account::{AccountIdentity, AccountRef, ProviderKind};
use headroom_core::cursor::LogCursors;
use headroom_core::event::UsageEvent;
use headroom_core::provider::{Provider, ProviderError};
use headroom_core::quota::{LimitsSnapshot, LimitsSource};
use jiff::Timestamp;

use self::client::UsageClient;
use self::identity::ClaudeIdentity;

pub use self::config::{ClaudeConfig, DEFAULT_API_BASE};

pub type Clock = fn() -> Timestamp;

#[derive(Debug, Clone)]
pub struct ClaudeProvider {
    config: ClaudeConfig,
    client: UsageClient,
    clock: Clock,
}

impl ClaudeProvider {
    pub fn new(config: ClaudeConfig) -> Result<ClaudeProvider, ProviderError> {
        ClaudeProvider::with_clock(config, Timestamp::now)
    }

    pub fn with_clock(config: ClaudeConfig, clock: Clock) -> Result<ClaudeProvider, ProviderError> {
        Ok(ClaudeProvider {
            client: UsageClient::new(&config.api_base)?,
            config,
            clock,
        })
    }

    fn current_identity(&self, account: &AccountRef) -> Result<ClaudeIdentity, ProviderError> {
        let identity = identity::load_identity(&self.config.home, &account.home)?
            .ok_or(ProviderError::NotSignedIn)?;
        if identity.account_id() == account.id {
            Ok(identity)
        } else {
            Err(ProviderError::LocalData(format!(
                "the Claude account signed in at {} has changed",
                account.home.display()
            )))
        }
    }
}

#[async_trait]
impl Provider for ClaudeProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Claude
    }

    async fn discover(&self) -> Result<Vec<AccountRef>, ProviderError> {
        Ok(accounts::discover_accounts(&self.config))
    }

    async fn fetch_limits(&self, account: &AccountRef) -> Result<LimitsSnapshot, ProviderError> {
        let identity = self.current_identity(account)?;
        let credentials = auth::load_credentials(&account.home)?;
        let now = (self.clock)();
        let raw = self
            .client
            .fetch(credentials.usable_token(now)?, now)
            .await?;
        let mapped = mapper::map_usage(&raw);
        Ok(LimitsSnapshot {
            identity: AccountIdentity {
                email: identity.email,
                plan: credentials.plan,
                stable_key: identity.stable_key,
            },
            windows: mapped.windows,
            balances: mapped.balances,
            notices: Vec::new(),
            fetched_at: now,
            source: LimitsSource::Live,
        })
    }

    fn read_usage(
        &self,
        home: &Path,
        cursors: &mut LogCursors,
    ) -> Result<Vec<UsageEvent>, ProviderError> {
        local_usage::read_usage(home, cursors)
    }
}

#[cfg(test)]
#[path = "provider_tests.rs"]
mod tests;
