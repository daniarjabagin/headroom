mod accounts;
mod auth;
mod client;
mod config;
mod mapper;
mod raw;
mod refresh;

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
use jiff::Timestamp;

use self::auth::{Credentials, Secret, parse_credentials, read_file};
use self::client::ClineClient;
use self::mapper::Fetched;
use self::raw::{RawOrganization, RawUser};
use self::refresh::{Stored, refresh_owned};
use crate::http;

pub use self::config::{ClineConfig, DEFAULT_API_BASE};

pub const ID: ProviderId = ProviderId::from_static("cline");

pub static DESCRIPTOR: ProviderDescriptor = ProviderDescriptor {
    id: ID,
    display_name: "Cline",
    add_account: &[
        AddAccountMethod::CliLogin(CliLogin {
            program: "cline",
            args: &["auth", "cline"],
            home_var: HomeVar::Direct("CLINE_DIR"),
            credentials_file: "data/settings/providers.json",
            default_dir: ".cline",
            needs_pty: true,
            scrub_env: &["CLINE_DATA_DIR", "CLINE_PROVIDER_SETTINGS_PATH"],
        }),
        AddAccountMethod::AutoDetect {
            reason: "the Cline CLI sign-in in ~/.cline is picked up automatically, but it expires \
                     about an hour after Cline last ran",
        },
    ],
    multi_account: true,
    local_usage: false,
    links: ProviderLinks {
        status: Some("https://status.cline.bot"),
        dashboard: Some("https://app.cline.bot/dashboard"),
        usage: Some("https://app.cline.bot/dashboard"),
    },
};

pub type Clock = fn() -> Timestamp;

#[derive(Debug, Clone)]
pub struct ClineProvider {
    config: ClineConfig,
    client: ClineClient,
    clock: Clock,
}

impl ClineProvider {
    #[must_use]
    pub fn with_http(config: ClineConfig, http: reqwest::Client) -> ClineProvider {
        ClineProvider::assemble(config, Timestamp::now, http)
    }

    pub fn with_clock(config: ClineConfig, clock: Clock) -> Result<ClineProvider, ProviderError> {
        Ok(ClineProvider::assemble(config, clock, http::client()?))
    }

    fn assemble(config: ClineConfig, clock: Clock, http: reqwest::Client) -> ClineProvider {
        ClineProvider {
            client: ClineClient::new(http, &config.api_base),
            config,
            clock,
        }
    }

    async fn access_token(
        &self,
        owner: CredentialOwner,
        stored: &Stored<'_>,
        credentials: &Credentials,
    ) -> Result<Secret, ProviderError> {
        if credentials.is_fresh((self.clock)()) {
            return Ok(Secret::new(credentials.access_token.expose().to_owned()));
        }
        match owner {
            CredentialOwner::Cli => Err(ProviderError::SignInExpired),
            CredentialOwner::Headroom => refresh_owned(&self.client, stored, credentials).await,
        }
    }

    async fn fetch_account(
        &self,
        token: &Secret,
        user: &RawUser,
    ) -> Result<Fetched, ProviderError> {
        let organization = active_organization(user);
        let organization_balance = async {
            match &organization {
                Some(org) => {
                    let balance = self
                        .client
                        .organization_balance(token, &org.organization_id);
                    balance.await.map(|balance| Some((org.clone(), balance)))
                }
                None => Ok(None),
            }
        };
        let (plan, balance, organization) = tokio::try_join!(
            self.client.plan(token),
            self.client.balance(token, &user.id),
            organization_balance
        )?;
        Ok(Fetched {
            plan,
            balance,
            organization,
        })
    }
}

fn active_organization(user: &RawUser) -> Option<RawOrganization> {
    user.organizations
        .iter()
        .flatten()
        .find(|organization| organization.active)
        .cloned()
}

fn check_account(credentials: &Credentials, account: &AccountRef) -> Result<(), ProviderError> {
    if credentials.account_id() == account.id {
        Ok(())
    } else {
        Err(ProviderError::AccountChanged(format!(
            "the Cline account signed in at {} has changed",
            account.home.display()
        )))
    }
}

#[async_trait]
impl Provider for ClineProvider {
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
        Ok(Vec::new())
    }

    async fn fetch_limits(&self, account: &AccountRef) -> Result<LimitsSnapshot, ProviderError> {
        let path = self.config.providers_file(&account.home, account.owner);
        let text = read_file(&path)?.ok_or(ProviderError::NotSignedIn)?;
        let credentials = parse_credentials(&text)?;
        check_account(&credentials, account)?;
        let stored = Stored {
            path: &path,
            text: &text,
        };
        let token = self
            .access_token(account.owner, &stored, &credentials)
            .await?;
        let user = self.client.me(&token).await?;
        let mapped = mapper::map_account(&self.fetch_account(&token, &user).await?)?;
        Ok(LimitsSnapshot {
            identity: AccountIdentity {
                email: user.email.or(credentials.email),
                plan: Some(mapped.plan),
                stable_key: credentials.user_id,
            },
            windows: Vec::new(),
            balances: mapped.balances,
            notices: mapped.notices,
            fetched_at: (self.clock)(),
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
#[path = "provider_tests.rs"]
mod tests;
