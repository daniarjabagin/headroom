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
mod subscription;
mod usage_homes;

use std::path::{Path, PathBuf};

use async_trait::async_trait;
use headroom_core::account::{AccountIdentity, AccountRef, ProviderId};
use headroom_core::cursor::LogCursors;
use headroom_core::descriptor::{AddAccountMethod, CliLogin, HomeVar, ProviderDescriptor};
use headroom_core::event::UsageEvent;
use headroom_core::provider::{Provider, ProviderError};
use headroom_core::quota::{LimitsSnapshot, LimitsSource};
use jiff::Timestamp;

use self::client::UsageClient;
use self::identity::ClaudeIdentity;
use crate::http;

pub use self::config::{ClaudeConfig, DEFAULT_API_BASE};

pub const ID: ProviderId = ProviderId::from_static("claude");

pub static DESCRIPTOR: ProviderDescriptor = ProviderDescriptor {
    id: ID,
    display_name: "Claude",
    add_account: &[AddAccountMethod::CliLogin(CliLogin {
        program: "claude",
        args: &["auth", "login", "--claudeai"],
        home_var: HomeVar::Direct("CLAUDE_CONFIG_DIR"),
        credentials_file: ".credentials.json",
        needs_pty: false,
        scrub_env: &[],
    })],
    multi_account: true,
    local_usage: true,
};

pub type Clock = fn() -> Timestamp;

#[derive(Debug, Clone)]
pub struct ClaudeProvider {
    config: ClaudeConfig,
    client: UsageClient,
    clock: Clock,
}

impl ClaudeProvider {
    pub fn new(config: ClaudeConfig) -> Result<ClaudeProvider, ProviderError> {
        Ok(ClaudeProvider::with_http(config, http::client()?))
    }

    #[must_use]
    pub fn with_http(config: ClaudeConfig, http: reqwest::Client) -> ClaudeProvider {
        ClaudeProvider::assemble(config, Timestamp::now, http)
    }

    pub fn with_clock(config: ClaudeConfig, clock: Clock) -> Result<ClaudeProvider, ProviderError> {
        Ok(ClaudeProvider::assemble(config, clock, http::client()?))
    }

    fn assemble(config: ClaudeConfig, clock: Clock, http: reqwest::Client) -> ClaudeProvider {
        ClaudeProvider {
            client: UsageClient::new(http, &config.api_base),
            config,
            clock,
        }
    }

    fn current_identity(&self, account: &AccountRef) -> Result<ClaudeIdentity, ProviderError> {
        let identity = identity::load_identity(&self.config, &account.home)?
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
    fn descriptor(&self) -> &'static ProviderDescriptor {
        &DESCRIPTOR
    }

    async fn discover(&self) -> Result<Vec<AccountRef>, ProviderError> {
        Ok(accounts::discover_accounts(&self.config))
    }

    async fn account_at(&self, home: &Path) -> Result<Option<AccountRef>, ProviderError> {
        accounts::headroom_account_at(&self.config, home)
    }

    async fn usage_homes(&self) -> Result<Vec<PathBuf>, ProviderError> {
        Ok(usage_homes::usage_homes(&self.config))
    }

    async fn fetch_limits(&self, account: &AccountRef) -> Result<LimitsSnapshot, ProviderError> {
        let identity = self.current_identity(account)?;
        let credentials = auth::load_credentials(&account.home)?;
        let now = (self.clock)();
        let fetched = match credentials.usable_token(now) {
            Ok(token) => self.client.fetch(token, now).await,
            Err(error) => Err(error),
        };
        let mapped = subscription::require_subscription(
            &credentials,
            fetched.map(|raw| mapper::map_usage(&raw)),
        )?;
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
