mod auth;
mod client;
mod config;
mod local_usage;
mod mapper;
mod refresh;
mod turn;

use std::path::{Path, PathBuf};

use async_trait::async_trait;
use headroom_core::account::{AccountRef, CredentialOwner, ProviderId};
use headroom_core::cursor::LogCursors;
use headroom_core::descriptor::{AddAccountMethod, CliLogin, HomeVar, ProviderDescriptor};
use headroom_core::event::UsageEvent;
use headroom_core::provider::{Provider, ProviderError};
use headroom_core::quota::LimitsSnapshot;
use jiff::Timestamp;

use self::auth::{Credentials, load_credentials};
use self::client::{GrokClient, RawBilling};
use self::mapper::PlanLookup;
use crate::http;

pub use self::config::{DEFAULT_API_BASE, DEFAULT_ISSUER, GrokConfig};

pub const ID: ProviderId = ProviderId::from_static("grok");

pub static DESCRIPTOR: ProviderDescriptor = ProviderDescriptor {
    id: ID,
    display_name: "Grok",
    add_account: &[
        AddAccountMethod::CliLogin(CliLogin {
            program: "grok",
            args: &["login"],
            home_var: HomeVar::Direct("GROK_HOME"),
            credentials_file: "auth.json",
            needs_pty: false,
            scrub_env: &["GROK_OIDC_ISSUER", "GROK_OIDC_CLIENT_ID"],
        }),
        AddAccountMethod::AutoDetect {
            reason: "Found from the Grok CLI sign-in in ~/.grok, which expires while the CLI is closed",
        },
    ],
    multi_account: true,
    local_usage: true,
};

pub type Clock = fn() -> Timestamp;

struct Usable {
    credentials: Credentials,
    refreshable: bool,
}

#[derive(Debug, Clone)]
pub struct GrokProvider {
    config: GrokConfig,
    client: GrokClient,
    clock: Clock,
}

impl GrokProvider {
    pub fn new(config: GrokConfig) -> Result<GrokProvider, ProviderError> {
        Ok(GrokProvider::with_http(config, http::client()?))
    }

    #[must_use]
    pub fn with_http(config: GrokConfig, http: reqwest::Client) -> GrokProvider {
        GrokProvider::assemble(config, Timestamp::now, http)
    }

    #[must_use]
    pub fn with_clock(config: GrokConfig, clock: Clock, http: reqwest::Client) -> GrokProvider {
        GrokProvider::assemble(config, clock, http)
    }

    fn assemble(config: GrokConfig, clock: Clock, http: reqwest::Client) -> GrokProvider {
        GrokProvider {
            client: GrokClient::new(http, &config.api_base),
            config,
            clock,
        }
    }

    async fn usable(
        &self,
        account: &AccountRef,
        credentials: Credentials,
        now: Timestamp,
    ) -> Result<Usable, ProviderError> {
        match account.owner {
            CredentialOwner::Cli if credentials.is_expired(now) => {
                Err(ProviderError::SignInExpired)
            }
            CredentialOwner::Headroom if credentials.expires_soon(now) => Ok(Usable {
                credentials: self.refresh(account, &credentials, now).await?,
                refreshable: false,
            }),
            CredentialOwner::Cli | CredentialOwner::Headroom => Ok(Usable {
                refreshable: account.owner == CredentialOwner::Headroom,
                credentials,
            }),
        }
    }

    async fn refresh(
        &self,
        account: &AccountRef,
        stale: &Credentials,
        now: Timestamp,
    ) -> Result<Credentials, ProviderError> {
        let issuer = &self.config.issuer;
        let fresh = refresh::refresh(&self.client, issuer, &account.home, stale, now).await?;
        ensure_same_account(account, &fresh)?;
        Ok(fresh)
    }

    async fn billing(
        &self,
        account: &AccountRef,
        usable: Usable,
        now: Timestamp,
    ) -> Result<(RawBilling, Credentials), ProviderError> {
        let Usable {
            credentials,
            refreshable,
        } = usable;
        match self.client.billing(&credentials.access_token).await {
            Err(ProviderError::SignInExpired) if refreshable => {
                let fresh = self.refresh(account, &credentials, now).await?;
                Ok((self.client.billing(&fresh.access_token).await?, fresh))
            }
            fetched => Ok((fetched?, credentials)),
        }
    }
}

#[async_trait]
impl Provider for GrokProvider {
    fn descriptor(&self) -> &'static ProviderDescriptor {
        &DESCRIPTOR
    }

    async fn discover(&self) -> Result<Vec<AccountRef>, ProviderError> {
        discover_accounts(self.config.homes()?)
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
        self.config.usage_homes()
    }

    async fn fetch_limits(&self, account: &AccountRef) -> Result<LimitsSnapshot, ProviderError> {
        let credentials = load_credentials(&account.home)?;
        ensure_same_account(account, &credentials)?;
        let now = (self.clock)();
        let usable = self.usable(account, credentials, now).await?;
        let (billing, credentials) = self.billing(account, usable, now).await?;
        let plan = PlanLookup::from_settings(self.client.settings(&credentials.access_token).await);
        mapper::map_limits(&billing, &plan, credentials.identity, now)
    }

    fn read_usage(
        &self,
        home: &Path,
        cursors: &mut LogCursors,
    ) -> Result<Vec<UsageEvent>, ProviderError> {
        local_usage::read_usage(home, cursors)
    }
}

fn ensure_same_account(
    account: &AccountRef,
    credentials: &Credentials,
) -> Result<(), ProviderError> {
    if credentials.identity.account_id(&ID) == account.id {
        Ok(())
    } else {
        Err(ProviderError::LocalData(format!(
            "the Grok account signed in at {} has changed",
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
            Ok(credentials) => accounts.push(account_ref(home, owner, &credentials)),
            Err(error) if owner == CredentialOwner::Cli => cli_error = Some(error),
            Err(error) => tracing::warn!(home = %home.display(), %error, "skipping grok home"),
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

#[cfg(test)]
#[path = "provider_tests.rs"]
mod tests;
