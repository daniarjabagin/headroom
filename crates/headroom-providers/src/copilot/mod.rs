mod accounts;
mod client;
mod config;
mod credits;
mod hosts;
mod mapper;
mod raw;
mod token;

use std::path::{Path, PathBuf};

use async_trait::async_trait;
use headroom_core::account::{AccountIdentity, AccountRef, ProviderId};
use headroom_core::cursor::LogCursors;
use headroom_core::descriptor::{
    AddAccountMethod, CliLogin, HomeVar, ProviderDescriptor, ProviderLinks,
};
use headroom_core::event::UsageEvent;
use headroom_core::provider::{Provider, ProviderError};
use headroom_core::quota::{LimitsSnapshot, LimitsSource};
use jiff::Timestamp;

use self::client::UserClient;

pub use self::config::{CopilotConfig, DEFAULT_API_BASE};

pub const ID: ProviderId = ProviderId::from_static("copilot");

pub static DESCRIPTOR: ProviderDescriptor = ProviderDescriptor {
    id: ID,
    display_name: "Copilot",
    add_account: &[
        AddAccountMethod::CliLogin(CliLogin {
            program: "gh",
            args: &[
                "auth",
                "login",
                "--web",
                "--hostname",
                hosts::GITHUB_HOST,
                "--git-protocol",
                "https",
                "--insecure-storage",
            ],
            home_var: HomeVar::Direct("GH_CONFIG_DIR"),
            credentials_file: hosts::HOSTS_FILE,
            needs_pty: false,
            scrub_env: &[
                "GH_TOKEN",
                "GITHUB_TOKEN",
                "GH_ENTERPRISE_TOKEN",
                "GITHUB_ENTERPRISE_TOKEN",
                "GH_HOST",
            ],
        }),
        AddAccountMethod::AutoDetect {
            reason: "Every account signed in to the GitHub CLI (gh) is found",
        },
    ],
    multi_account: true,
    local_usage: false,
    links: ProviderLinks {
        status: Some("https://www.githubstatus.com"),
        dashboard: Some("https://github.com/settings/copilot"),
        usage: Some("https://github.com/settings/billing/summary"),
    },
};

pub type Clock = fn() -> Timestamp;

#[derive(Debug, Clone)]
pub struct CopilotProvider {
    config: CopilotConfig,
    client: UserClient,
    clock: Clock,
}

impl CopilotProvider {
    #[must_use]
    pub fn with_http(config: CopilotConfig, http: reqwest::Client) -> CopilotProvider {
        CopilotProvider::with_clock(config, http, Timestamp::now)
    }

    #[must_use]
    pub fn with_clock(
        config: CopilotConfig,
        http: reqwest::Client,
        clock: Clock,
    ) -> CopilotProvider {
        CopilotProvider {
            client: UserClient::new(http, &config.api_base),
            config,
            clock,
        }
    }
}

#[async_trait]
impl Provider for CopilotProvider {
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
        let login = accounts::current_gh_login(account)?;
        let token = token::gh_token(&self.config.gh_program, &account.home, &login).await?;
        let now = (self.clock)();
        let raw = self.client.fetch(&token, now).await?;
        let mapped = mapper::map_user(&raw)?;
        Ok(LimitsSnapshot {
            identity: AccountIdentity {
                stable_key: accounts::stable_key(&login),
                email: Some(login),
                plan: mapped.plan,
            },
            windows: mapped.windows,
            balances: mapped.balances,
            notices: mapped.notices,
            fetched_at: now,
            source: LimitsSource::Live,
        })
    }

    fn read_usage(&self, _: &Path, _: &mut LogCursors) -> Result<Vec<UsageEvent>, ProviderError> {
        Ok(Vec::new())
    }
}

#[cfg(test)]
mod test_support;

#[cfg(test)]
#[path = "provider_tests.rs"]
mod tests;
