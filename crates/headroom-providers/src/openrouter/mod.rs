mod client;
mod mapper;
mod money;
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

use self::client::KeyClient;
use crate::key_accounts;
use crate::paths::HeadroomDirs;

pub const ID: ProviderId = ProviderId::from_static("openrouter");
pub const DEFAULT_API_BASE: &str = "https://openrouter.ai";
const CONSOLE_URL: &str = "https://openrouter.ai/settings/keys";

pub static DESCRIPTOR: ProviderDescriptor = ProviderDescriptor {
    id: ID,
    display_name: "OpenRouter",
    add_account: &[AddAccountMethod::ApiKey(ApiKeyPrompt {
        label: "API key",
        console_url: CONSOLE_URL,
        hint: "Starts with sk-or-; a management key also shows the credit balance",
    })],
    multi_account: true,
    local_usage: false,
    min_poll_interval: None,
    links: ProviderLinks {
        status: Some("https://status.openrouter.ai"),
        dashboard: Some("https://openrouter.ai/settings/credits"),
        usage: Some("https://openrouter.ai/activity"),
    },
};

pub type Clock = fn() -> Timestamp;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenRouterConfig {
    pub accounts_dir: PathBuf,
    pub api_base: String,
}

impl OpenRouterConfig {
    pub fn from_process() -> Result<OpenRouterConfig, ProviderError> {
        let dirs = HeadroomDirs::from_process()
            .ok_or_else(|| ProviderError::LocalData("home directory not found".to_owned()))?;
        Ok(OpenRouterConfig {
            accounts_dir: dirs.accounts(ID.as_str()),
            api_base: DEFAULT_API_BASE.to_owned(),
        })
    }
}

pub struct OpenRouterProvider {
    accounts_dir: PathBuf,
    client: KeyClient,
    secrets: Arc<dyn SecretReader>,
    clock: Clock,
}

impl OpenRouterProvider {
    #[must_use]
    pub fn new(
        config: OpenRouterConfig,
        http: reqwest::Client,
        secrets: Arc<dyn SecretReader>,
    ) -> OpenRouterProvider {
        OpenRouterProvider {
            client: KeyClient::new(http, &config.api_base),
            accounts_dir: config.accounts_dir,
            secrets,
            clock: Timestamp::now,
        }
    }

    #[must_use]
    pub fn with_clock(self, clock: Clock) -> OpenRouterProvider {
        OpenRouterProvider { clock, ..self }
    }
}

#[async_trait]
impl Provider for OpenRouterProvider {
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
        let (raw_key, credits) =
            tokio::join!(self.client.key(key, now), self.client.credits(key, now));
        let mapped = mapper::map(&raw_key?, &credits);
        Ok(LimitsSnapshot {
            identity: identity(key, mapped.plan),
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

    async fn validate_key(&self, key: &str) -> Result<AccountIdentity, ProviderError> {
        let raw = self
            .client
            .key(key, (self.clock)())
            .await
            .map_err(rejected_key)?;
        Ok(identity(key, mapper::plan(&raw)))
    }
}

fn identity(key: &str, plan: Option<String>) -> AccountIdentity {
    AccountIdentity {
        email: None,
        plan,
        stable_key: key_accounts::sha256_stable_key(key),
    }
}

fn rejected_key(error: ProviderError) -> ProviderError {
    match error {
        ProviderError::SignInExpired => ProviderError::Unsupported(format!(
            "OpenRouter rejected this API key; check it at {CONSOLE_URL}"
        )),
        other => other,
    }
}

#[cfg(test)]
#[path = "provider_tests.rs"]
mod tests;
