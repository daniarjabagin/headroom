mod accounts;
mod auth;
mod client;
mod config;
mod credentials;
mod identity;
mod keychain;
mod local_usage;
mod log_record;
mod mapper;
mod number;
mod oauth;
mod raw;
mod refresh;
mod subscription;
mod usage_homes;

use std::path::{Path, PathBuf};

use async_trait::async_trait;
use headroom_core::account::{AccountIdentity, AccountRef, CredentialOwner, ProviderId};
use headroom_core::cursor::LogCursors;
use headroom_core::descriptor::{
    AddAccountMethod, CliLogin, HomeVar, ProviderDescriptor, ProviderLinks,
};
use headroom_core::event::UsageEvent;
use headroom_core::provider::{Provider, ProviderError};
use headroom_core::quota::{LimitsSnapshot, LimitsSource};
use jiff::{SignedDuration, Timestamp};

use self::auth::Credentials;
use self::client::UsageClient;
use self::identity::ClaudeIdentity;
use self::oauth::TokenClient;
use self::raw::RawUsage;
use crate::http;

pub use self::config::{ClaudeConfig, DEFAULT_API_BASE};
pub use self::oauth::DEFAULT_TOKEN_URL;

const MIN_POLL_INTERVAL: SignedDuration = SignedDuration::from_secs(180);

pub const ID: ProviderId = ProviderId::from_static("claude");

pub static DESCRIPTOR: ProviderDescriptor = ProviderDescriptor {
    id: ID,
    display_name: "Claude",
    add_account: &[AddAccountMethod::CliLogin(CliLogin {
        program: "claude",
        args: &["auth", "login", "--claudeai"],
        home_var: HomeVar::Direct("CLAUDE_CONFIG_DIR"),
        credentials_file: ".credentials.json",
        default_dir: ".claude",
        needs_pty: false,
        scrub_env: &[],
    })],
    multi_account: true,
    local_usage: true,
    min_poll_interval: Some(MIN_POLL_INTERVAL),
    links: ProviderLinks {
        status: Some("https://status.claude.com"),
        dashboard: Some("https://claude.ai"),
        usage: None,
    },
};

pub type Clock = fn() -> Timestamp;

#[derive(Debug, Clone)]
pub struct ClaudeProvider {
    config: ClaudeConfig,
    client: UsageClient,
    tokens: TokenClient,
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
            client: UsageClient::new(http.clone(), &config.api_base),
            tokens: TokenClient::new(http, &config.token_url),
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
            Err(ProviderError::AccountChanged(format!(
                "the Claude account signed in at {} has changed",
                account.home.display()
            )))
        }
    }
}

impl ClaudeProvider {
    fn refreshable(&self, account: &AccountRef) -> bool {
        account.owner == CredentialOwner::Headroom && self.config.keychain.is_none()
    }

    async fn fetch_usage(
        &self,
        credentials: &Credentials,
        now: Timestamp,
    ) -> Result<RawUsage, ProviderError> {
        let token = credentials.usable_token(now)?;
        self.client.fetch(token, now).await
    }

    async fn fetch_refreshing(
        &self,
        account: &AccountRef,
        credentials: Credentials,
        now: Timestamp,
    ) -> (Credentials, Result<RawUsage, ProviderError>) {
        match self.fetch_usage(&credentials, now).await {
            Err(ProviderError::SignInExpired)
                if self.refreshable(account) && credentials.has_profile_scope() =>
            {
                match refresh::refresh(&self.tokens, &account.home, &credentials, now).await {
                    Ok(fresh) => {
                        let fetched = self.fetch_usage(&fresh, now).await;
                        (fresh, fetched)
                    }
                    Err(error) => (credentials, Err(error)),
                }
            }
            fetched => (credentials, fetched),
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
        let credentials = credentials::load(&self.config, account).await?;
        let now = (self.clock)();
        let (credentials, fetched) = self.fetch_refreshing(account, credentials, now).await;
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

#[cfg(test)]
#[path = "owned_refresh_tests.rs"]
mod owned_refresh_tests;
