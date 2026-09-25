mod auth;
mod client;
mod config;
mod extras;
mod jwt;
mod limits;
mod mapper;
mod number;
mod raw;
mod state_db;

use std::path::{Path, PathBuf};

use async_trait::async_trait;
use headroom_core::account::{AccountId, AccountIdentity, AccountRef, CredentialOwner, ProviderId};
use headroom_core::cursor::LogCursors;
use headroom_core::descriptor::{AddAccountMethod, ProviderDescriptor};
use headroom_core::event::UsageEvent;
use headroom_core::provider::{Provider, ProviderError};
use headroom_core::quota::{LimitsSnapshot, LimitsSource};
use jiff::Timestamp;

use self::auth::Credentials;
use self::client::CursorClient;
use crate::http;

pub use self::config::{CursorConfig, DEFAULT_API_BASE, DEFAULT_WEB_BASE};

pub const ID: ProviderId = ProviderId::from_static("cursor");

pub static DESCRIPTOR: ProviderDescriptor = ProviderDescriptor {
    id: ID,
    display_name: "Cursor",
    add_account: &[AddAccountMethod::AutoDetect {
        reason: "Cursor keeps one login per machine: sign in to the Cursor app (or run `agent login`) and Headroom picks it up.",
    }],
    multi_account: false,
    local_usage: false,
};

pub type Clock = fn() -> Timestamp;

#[derive(Debug, Clone)]
pub struct CursorProvider {
    config: CursorConfig,
    client: CursorClient,
    clock: Clock,
}

impl CursorProvider {
    pub fn new(config: CursorConfig) -> Result<CursorProvider, ProviderError> {
        Ok(CursorProvider::with_http(config, http::client()?))
    }

    #[must_use]
    pub fn with_http(config: CursorConfig, http: reqwest::Client) -> CursorProvider {
        CursorProvider::assemble(config, Timestamp::now, http)
    }

    pub fn with_clock(config: CursorConfig, clock: Clock) -> Result<CursorProvider, ProviderError> {
        Ok(CursorProvider::assemble(config, clock, http::client()?))
    }

    fn assemble(config: CursorConfig, clock: Clock, http: reqwest::Client) -> CursorProvider {
        CursorProvider {
            client: CursorClient::new(http, &config),
            config,
            clock,
        }
    }

    fn account_ref(&self, credentials: &Credentials) -> AccountRef {
        AccountRef {
            id: AccountId::from_stable_key(&ID, &credentials.subject),
            provider: ID,
            home: credentials.home(&self.config),
            owner: CredentialOwner::Cli,
        }
    }

    fn current_credentials(&self, account: &AccountRef) -> Result<Credentials, ProviderError> {
        let credentials =
            auth::load_credentials(&self.config)?.ok_or(ProviderError::NotSignedIn)?;
        if self.account_ref(&credentials).id == account.id {
            Ok(credentials)
        } else {
            Err(ProviderError::AccountChanged(
                "the Cursor account signed in on this machine has changed".to_owned(),
            ))
        }
    }
}

#[async_trait]
impl Provider for CursorProvider {
    fn descriptor(&self) -> &'static ProviderDescriptor {
        &DESCRIPTOR
    }

    async fn discover(&self) -> Result<Vec<AccountRef>, ProviderError> {
        let credentials = auth::load_credentials(&self.config)?;
        Ok(credentials.iter().map(|c| self.account_ref(c)).collect())
    }

    async fn usage_homes(&self) -> Result<Vec<PathBuf>, ProviderError> {
        Ok(Vec::new())
    }

    async fn fetch_limits(&self, account: &AccountRef) -> Result<LimitsSnapshot, ProviderError> {
        let credentials = self.current_credentials(account)?;
        let now = (self.clock)();
        let token = credentials.usable_token(now)?;
        let fetched = limits::fetch(&self.client, &credentials, token, now).await?;
        Ok(LimitsSnapshot {
            identity: AccountIdentity {
                email: credentials.email.clone(),
                plan: fetched.plan,
                stable_key: credentials.subject.clone(),
            },
            windows: fetched.usage.windows,
            balances: fetched.usage.balances,
            notices: Vec::new(),
            fetched_at: now,
            source: LimitsSource::Live,
        })
    }

    fn read_usage(
        &self,
        _home: &Path,
        _cursors: &mut LogCursors,
    ) -> Result<Vec<UsageEvent>, ProviderError> {
        Ok(Vec::new())
    }
}

#[cfg(test)]
mod test_support;

#[cfg(test)]
#[path = "provider_tests.rs"]
mod tests;
