mod client;
mod mapper;

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

use self::client::PoeClient;
use crate::key_accounts;
use crate::paths::HeadroomDirs;

pub const ID: ProviderId = ProviderId::from_static("poe");
pub const DEFAULT_API_BASE: &str = "https://api.poe.com";
const CONSOLE_URL: &str = "https://poe.com/api/keys";

pub static DESCRIPTOR: ProviderDescriptor = ProviderDescriptor {
    id: ID,
    display_name: "Poe",
    add_account: &[AddAccountMethod::ApiKey(ApiKeyPrompt {
        label: "Poe API key",
        console_url: CONSOLE_URL,
        hint: "Shows the point balance of the account that owns the key",
    })],
    multi_account: true,
    local_usage: false,
    links: ProviderLinks {
        status: Some("https://status.poe.com"),
        dashboard: Some("https://poe.com/settings"),
        usage: Some("https://poe.com/points_history"),
    },
};

pub type Clock = fn() -> Timestamp;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PoeConfig {
    pub accounts_dir: PathBuf,
    pub api_base: String,
}

impl PoeConfig {
    pub fn from_process() -> Result<PoeConfig, ProviderError> {
        let dirs = HeadroomDirs::from_process()
            .ok_or_else(|| ProviderError::LocalData("home directory not found".to_owned()))?;
        Ok(PoeConfig {
            accounts_dir: dirs.accounts(ID.as_str()),
            api_base: DEFAULT_API_BASE.to_owned(),
        })
    }
}

pub struct PoeProvider {
    accounts_dir: PathBuf,
    client: PoeClient,
    secrets: Arc<dyn SecretReader>,
    clock: Clock,
}

impl PoeProvider {
    #[must_use]
    pub fn new(
        config: PoeConfig,
        http: reqwest::Client,
        secrets: Arc<dyn SecretReader>,
    ) -> PoeProvider {
        PoeProvider {
            client: PoeClient::new(http, &config.api_base),
            accounts_dir: config.accounts_dir,
            secrets,
            clock: Timestamp::now,
        }
    }

    #[must_use]
    pub fn with_clock(self, clock: Clock) -> PoeProvider {
        PoeProvider { clock, ..self }
    }
}

#[async_trait]
impl Provider for PoeProvider {
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
        let now = (self.clock)();
        let raw = self.client.balance(secret.expose(), now).await?;
        Ok(LimitsSnapshot {
            identity: identity(secret.expose()),
            windows: Vec::new(),
            balances: vec![mapper::point_balance(&raw)],
            notices: Vec::new(),
            fetched_at: now,
            source: LimitsSource::Live,
        })
    }

    fn read_usage(&self, _: &Path, _: &mut LogCursors) -> Result<Vec<UsageEvent>, ProviderError> {
        Ok(Vec::new())
    }

    async fn validate_key(&self, key: &str) -> Result<AccountIdentity, ProviderError> {
        self.client
            .balance(key, (self.clock)())
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
            "Poe rejected this API key; check it at {CONSOLE_URL}"
        )),
        other => other,
    }
}

#[cfg(test)]
#[path = "provider_tests.rs"]
mod tests;
