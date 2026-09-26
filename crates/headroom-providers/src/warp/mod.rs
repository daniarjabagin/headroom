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

use self::client::WarpClient;
use crate::key_accounts;
use crate::paths::HeadroomDirs;

pub const ID: ProviderId = ProviderId::from_static("warp");
pub const DEFAULT_API_BASE: &str = "https://app.warp.dev";
const CONSOLE_URL: &str = "https://docs.warp.dev/reference/cli/api-keys";

pub static DESCRIPTOR: ProviderDescriptor = ProviderDescriptor {
    id: ID,
    display_name: "Warp",
    add_account: &[AddAccountMethod::ApiKey(ApiKeyPrompt {
        label: "Warp API key",
        console_url: CONSOLE_URL,
        hint: "Starts with wk-; create one in the Warp app settings or with `oz api-key create`",
    })],
    multi_account: true,
    local_usage: false,
    min_poll_interval: None,
    links: ProviderLinks {
        status: Some("https://status.warp.dev"),
        dashboard: Some("https://app.warp.dev"),
        usage: None,
    },
};

pub type Clock = fn() -> Timestamp;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WarpConfig {
    pub accounts_dir: PathBuf,
    pub api_base: String,
}

impl WarpConfig {
    pub fn from_process() -> Result<WarpConfig, ProviderError> {
        let dirs = HeadroomDirs::from_process()
            .ok_or_else(|| ProviderError::LocalData("home directory not found".to_owned()))?;
        Ok(WarpConfig {
            accounts_dir: dirs.accounts(ID.as_str()),
            api_base: DEFAULT_API_BASE.to_owned(),
        })
    }
}

pub struct WarpProvider {
    accounts_dir: PathBuf,
    client: WarpClient,
    secrets: Arc<dyn SecretReader>,
    clock: Clock,
}

impl WarpProvider {
    #[must_use]
    pub fn new(
        config: WarpConfig,
        http: reqwest::Client,
        secrets: Arc<dyn SecretReader>,
    ) -> WarpProvider {
        WarpProvider {
            client: WarpClient::new(http, &config.api_base),
            accounts_dir: config.accounts_dir,
            secrets,
            clock: Timestamp::now,
        }
    }

    #[must_use]
    pub fn with_clock(self, clock: Clock) -> WarpProvider {
        WarpProvider { clock, ..self }
    }
}

#[async_trait]
impl Provider for WarpProvider {
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
        let raw = self.client.request_limits(secret.expose(), now).await?;
        let mapped = mapper::map(&raw)?;
        Ok(LimitsSnapshot {
            identity: identity(secret.expose()),
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
        self.client
            .request_limits(key, (self.clock)())
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
            "Warp rejected this API key; check it at {CONSOLE_URL}"
        )),
        other => other,
    }
}

#[cfg(test)]
#[path = "provider_tests.rs"]
mod tests;
