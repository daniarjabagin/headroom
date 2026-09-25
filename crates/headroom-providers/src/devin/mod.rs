mod accounts;
mod auth;
mod client;
mod config;
mod mapper;
mod raw;
mod state_db;

use std::path::{Path, PathBuf};

use async_trait::async_trait;
use headroom_core::account::{AccountIdentity, AccountRef, ProviderId};
use headroom_core::cursor::LogCursors;
use headroom_core::descriptor::{AddAccountMethod, CliLogin, HomeVar, ProviderDescriptor};
use headroom_core::event::UsageEvent;
use headroom_core::provider::{Provider, ProviderError};
use headroom_core::quota::{LimitsSnapshot, LimitsSource};
use jiff::Timestamp;

use self::auth::{DevinKey, load_key};
use self::client::StatusClient;

pub use self::config::{DEFAULT_API_BASE, DevinConfig};

pub const ID: ProviderId = ProviderId::from_static("devin");
const DATA_SUBDIR: &str = "devin";

pub static DESCRIPTOR: ProviderDescriptor = ProviderDescriptor {
    id: ID,
    display_name: "Devin",
    add_account: &[
        AddAccountMethod::CliLogin(CliLogin {
            program: "devin",
            args: &["auth", "login"],
            home_var: HomeVar::XdgBase {
                var: "XDG_DATA_HOME",
                subdir: DATA_SUBDIR,
            },
            credentials_file: auth::CREDENTIALS_FILE,
            needs_pty: false,
            scrub_env: &[],
        }),
        AddAccountMethod::AutoDetect {
            reason: "Found when you sign in with the Devin CLI or the Devin app",
        },
    ],
    multi_account: true,
    local_usage: false,
};

pub type Clock = fn() -> Timestamp;

#[derive(Debug, Clone)]
pub struct DevinProvider {
    config: DevinConfig,
    client: StatusClient,
    clock: Clock,
}

impl DevinProvider {
    #[must_use]
    pub fn with_http(config: DevinConfig, http: reqwest::Client) -> DevinProvider {
        DevinProvider::with_clock(config, http, Timestamp::now)
    }

    #[must_use]
    pub fn with_clock(config: DevinConfig, http: reqwest::Client, clock: Clock) -> DevinProvider {
        DevinProvider {
            config,
            client: StatusClient::new(http),
            clock,
        }
    }

    fn api_base<'a>(&'a self, key: &'a DevinKey) -> &'a str {
        key.api_server.as_deref().unwrap_or(&self.config.api_base)
    }
}

fn current_key(account: &AccountRef) -> Result<DevinKey, ProviderError> {
    let key = load_key(&accounts::key_dir(&account.home, account.owner))?
        .ok_or(ProviderError::NotSignedIn)?;
    if key.account_id() == account.id {
        Ok(key)
    } else {
        Err(ProviderError::AccountChanged(format!(
            "the Devin account signed in at {} has changed",
            account.home.display()
        )))
    }
}

#[async_trait]
impl Provider for DevinProvider {
    fn descriptor(&self) -> &'static ProviderDescriptor {
        &DESCRIPTOR
    }

    async fn discover(&self) -> Result<Vec<AccountRef>, ProviderError> {
        Ok(accounts::discover_accounts(&self.config))
    }

    async fn account_at(&self, home: &Path) -> Result<Option<AccountRef>, ProviderError> {
        accounts::headroom_account_at(home)
    }

    async fn usage_homes(&self) -> Result<Vec<PathBuf>, ProviderError> {
        Ok(Vec::new())
    }

    async fn fetch_limits(&self, account: &AccountRef) -> Result<LimitsSnapshot, ProviderError> {
        let key = current_key(account)?;
        let now = (self.clock)();
        let raw = self.client.fetch(self.api_base(&key), &key, now).await?;
        let mapped = mapper::map_status(&raw.user_status)?;
        Ok(LimitsSnapshot {
            identity: AccountIdentity {
                email: mapped.email,
                plan: mapped.plan,
                stable_key: key.stable_key(),
            },
            windows: mapped.windows,
            balances: mapped.balances,
            notices: Vec::new(),
            fetched_at: now,
            source: LimitsSource::Live,
        })
    }

    fn read_usage(&self, _: &Path, _: &mut LogCursors) -> Result<Vec<UsageEvent>, ProviderError> {
        Ok(Vec::new())
    }
}

#[cfg(test)]
#[path = "provider_tests.rs"]
mod tests;
