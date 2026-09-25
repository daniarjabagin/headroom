mod client;
mod mapper;
mod raw;

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
use crate::key_accounts;
use crate::paths::HeadroomDirs;

pub const ID: ProviderId = ProviderId::from_static("deepseek");
pub const DEFAULT_API_BASE: &str = "https://api.deepseek.com";
const CONSOLE_URL: &str = "https://platform.deepseek.com/api_keys";

pub static DESCRIPTOR: ProviderDescriptor = ProviderDescriptor {
    id: ID,
    display_name: "DeepSeek",
    add_account: &[AddAccountMethod::ApiKey(ApiKeyPrompt {
        label: "DeepSeek API key",
        console_url: CONSOLE_URL,
        hint: "Any key of the account; the balance is shown in each currency it holds",
    })],
    multi_account: true,
    local_usage: false,
    links: ProviderLinks {
        status: Some("https://status.deepseek.com"),
        dashboard: Some("https://platform.deepseek.com"),
        usage: Some("https://platform.deepseek.com/usage"),
    },
};

pub type Clock = fn() -> Timestamp;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeepSeekConfig {
    pub accounts_dir: PathBuf,
    pub api_base: String,
}

impl DeepSeekConfig {
    pub fn from_process() -> Result<DeepSeekConfig, ProviderError> {
        let dirs = HeadroomDirs::from_process()
            .ok_or_else(|| ProviderError::LocalData("home directory not found".to_owned()))?;
        Ok(DeepSeekConfig {
            accounts_dir: dirs.accounts(ID.as_str()),
            api_base: DEFAULT_API_BASE.to_owned(),
        })
    }
}

pub struct DeepSeekProvider {
    accounts_dir: PathBuf,
    client: BalanceClient,
    secrets: Arc<dyn SecretReader>,
    clock: Clock,
}

impl DeepSeekProvider {
    #[must_use]
    pub fn new(
        config: DeepSeekConfig,
        http: reqwest::Client,
        secrets: Arc<dyn SecretReader>,
    ) -> DeepSeekProvider {
        DeepSeekProvider {
            client: BalanceClient::new(http, &config.api_base),
            accounts_dir: config.accounts_dir,
            secrets,
            clock: Timestamp::now,
        }
    }

    #[must_use]
    pub fn with_clock(self, clock: Clock) -> DeepSeekProvider {
        DeepSeekProvider { clock, ..self }
    }
}

#[async_trait]
impl Provider for DeepSeekProvider {
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
        let mapped = mapper::map(&self.client.balance(key, now).await?)?;
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
        let raw = self
            .client
            .balance(key, (self.clock)())
            .await
            .map_err(rejected_key)?;
        mapper::map(&raw)?;
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
            "DeepSeek rejected this API key; check it at {CONSOLE_URL}"
        )),
        other => other,
    }
}

#[cfg(test)]
#[path = "provider_tests.rs"]
mod tests;
