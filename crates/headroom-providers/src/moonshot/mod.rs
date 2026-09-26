mod client;
mod mapper;
mod raw;
mod region;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use headroom_core::account::{AccountIdentity, AccountRef, ProviderId};
use headroom_core::cursor::LogCursors;
use headroom_core::descriptor::{
    AddAccountMethod, ApiKeyPrompt, ProviderDescriptor, ProviderLinks,
};
use headroom_core::event::UsageEvent;
use headroom_core::provider::{Provider, ProviderError};
use headroom_core::quota::{LimitsSnapshot, LimitsSource};
use headroom_core::secret::SecretReader;
use jiff::Timestamp;

use self::client::BalanceClient;
use self::region::{KnownRegions, Region};
use crate::key_accounts;
use crate::paths::HeadroomDirs;

pub const ID: ProviderId = ProviderId::from_static("moonshot");
pub const GLOBAL_API_BASE: &str = "https://api.moonshot.ai";
pub const MAINLAND_API_BASE: &str = "https://api.moonshot.cn";
const CONSOLE_URL: &str = "https://platform.kimi.ai/console/api-keys";

pub static DESCRIPTOR: ProviderDescriptor = ProviderDescriptor {
    id: ID,
    display_name: "Moonshot API",
    add_account: &[AddAccountMethod::ApiKey(ApiKeyPrompt {
        label: "Moonshot API key",
        console_url: CONSOLE_URL,
        hint: "A pay-as-you-go key from platform.kimi.ai (USD) or platform.kimi.com (CNY); \
               Kimi Code keys belong to Kimi Code",
    })],
    multi_account: true,
    local_usage: false,
    min_poll_interval: None,
    links: ProviderLinks {
        status: Some("https://status.moonshot.cn"),
        dashboard: Some("https://platform.kimi.ai/console"),
        usage: Some("https://platform.kimi.ai/console/account"),
    },
};

pub type Clock = fn() -> Timestamp;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MoonshotConfig {
    pub accounts_dir: PathBuf,
    pub global_api_base: String,
    pub mainland_api_base: String,
}

impl MoonshotConfig {
    pub fn from_process() -> Result<MoonshotConfig, ProviderError> {
        let dirs = HeadroomDirs::from_process()
            .ok_or_else(|| ProviderError::LocalData("home directory not found".to_owned()))?;
        Ok(MoonshotConfig {
            accounts_dir: dirs.accounts(ID.as_str()),
            global_api_base: GLOBAL_API_BASE.to_owned(),
            mainland_api_base: MAINLAND_API_BASE.to_owned(),
        })
    }
}

pub struct MoonshotProvider {
    accounts_dir: PathBuf,
    client: BalanceClient,
    regions: KnownRegions,
    secrets: Arc<dyn SecretReader>,
    clock: Clock,
}

impl MoonshotProvider {
    #[must_use]
    pub fn new(
        config: MoonshotConfig,
        http: reqwest::Client,
        secrets: Arc<dyn SecretReader>,
    ) -> MoonshotProvider {
        MoonshotProvider {
            client: BalanceClient::new(http, &config.global_api_base, &config.mainland_api_base),
            accounts_dir: config.accounts_dir,
            regions: KnownRegions::default(),
            secrets,
            clock: Timestamp::now,
        }
    }

    #[must_use]
    pub fn with_clock(self, clock: Clock) -> MoonshotProvider {
        MoonshotProvider { clock, ..self }
    }
}

#[async_trait]
impl Provider for MoonshotProvider {
    fn descriptor(&self) -> &'static ProviderDescriptor {
        &DESCRIPTOR
    }

    async fn discover(&self) -> Result<Vec<AccountRef>, ProviderError> {
        key_accounts::discover(&self.accounts_dir, &ID)
    }

    async fn usage_homes(&self) -> Result<Vec<PathBuf>, ProviderError> {
        Ok(Vec::new())
    }

    async fn fetch_limits(&self, account: &AccountRef) -> Result<LimitsSnapshot, ProviderError> {
        let secret = key_accounts::stored_key(self.secrets.as_ref(), account).await?;
        let key = secret.expose();
        let now = (self.clock)();
        let first = self.regions.first_try(&account.id);
        let (region, raw) = self.client.locate(first, key, now).await?;
        self.regions.remember(&account.id, region);
        let mapped = mapper::map(region, &raw)?;
        Ok(LimitsSnapshot {
            identity: identity(key),
            windows: Vec::new(),
            balances: mapped.balances,
            notices: mapped.notices,
            fetched_at: now,
            source: LimitsSource::Live,
        })
    }

    fn read_usage(&self, _: &Path, _: &mut LogCursors) -> Result<Vec<UsageEvent>, ProviderError> {
        Ok(Vec::new())
    }

    async fn validate_key(&self, key: &str) -> Result<AccountIdentity, ProviderError> {
        self.client
            .locate(Region::Global, key, (self.clock)())
            .await
            .map_err(rejected_key)?;
        Ok(identity(key))
    }
}

fn identity(key: &str) -> AccountIdentity {
    AccountIdentity {
        email: None,
        plan: None,
        stable_key: key_accounts::sha256_stable_key(key),
    }
}

fn rejected_key(error: ProviderError) -> ProviderError {
    match error {
        ProviderError::SignInExpired => ProviderError::Unsupported(format!(
            "Moonshot rejected this API key on api.moonshot.ai and api.moonshot.cn; \
             check it at {CONSOLE_URL}"
        )),
        other => other,
    }
}

#[cfg(test)]
#[path = "provider_tests.rs"]
mod tests;
