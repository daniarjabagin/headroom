mod client;
mod mapper;
mod raw;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use headroom_core::account::{AccountIdentity, AccountRef, ProviderId};
use headroom_core::cursor::LogCursors;
use headroom_core::descriptor::{AddAccountMethod, ApiKeyPrompt, ProviderDescriptor};
use headroom_core::event::UsageEvent;
use headroom_core::provider::{Provider, ProviderError};
use headroom_core::quota::{LimitsSnapshot, LimitsSource, QuotaWindow};
use headroom_core::secret::SecretReader;
use jiff::Timestamp;

use self::client::{QuotaClient, Scheme};
use crate::key_accounts;
use crate::paths::HeadroomDirs;

pub const ID: ProviderId = ProviderId::from_static("zai");
pub const DEFAULT_API_BASE: &str = "https://api.z.ai";
const CONSOLE_URL: &str = "https://z.ai/manage-apikey/apikey-list";

pub static DESCRIPTOR: ProviderDescriptor = ProviderDescriptor {
    id: ID,
    display_name: "Z.ai",
    add_account: &[AddAccountMethod::ApiKey(ApiKeyPrompt {
        label: "API key",
        console_url: CONSOLE_URL,
        hint: "A key from the account that holds your GLM Coding Plan",
    })],
    multi_account: true,
    local_usage: false,
};

pub type Clock = fn() -> Timestamp;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZaiConfig {
    pub accounts_dir: PathBuf,
    pub api_base: String,
}

impl ZaiConfig {
    pub fn from_process() -> Result<ZaiConfig, ProviderError> {
        let dirs = HeadroomDirs::from_process()
            .ok_or_else(|| ProviderError::LocalData("home directory not found".to_owned()))?;
        Ok(ZaiConfig {
            accounts_dir: dirs.accounts(ID.as_str()),
            api_base: DEFAULT_API_BASE.to_owned(),
        })
    }
}

pub struct ZaiProvider {
    accounts_dir: PathBuf,
    client: QuotaClient,
    secrets: Arc<dyn SecretReader>,
    clock: Clock,
}

struct Quota {
    windows: Vec<QuotaWindow>,
    plan: Option<String>,
}

impl ZaiProvider {
    #[must_use]
    pub fn new(
        config: ZaiConfig,
        http: reqwest::Client,
        secrets: Arc<dyn SecretReader>,
    ) -> ZaiProvider {
        ZaiProvider {
            client: QuotaClient::new(http, &config.api_base),
            accounts_dir: config.accounts_dir,
            secrets,
            clock: Timestamp::now,
        }
    }

    #[must_use]
    pub fn with_clock(self, clock: Clock) -> ZaiProvider {
        ZaiProvider { clock, ..self }
    }

    async fn quota(&self, key: &str, now: Timestamp) -> Result<Quota, ProviderError> {
        let (body, scheme) = self.client.quota(key, now).await?;
        let windows = mapper::windows(&mapper::quota_limits(&body)?)?;
        let plan = self.plan(key, scheme, now).await;
        Ok(Quota { windows, plan })
    }

    async fn plan(&self, key: &str, scheme: Scheme, now: Timestamp) -> Option<String> {
        match self.client.subscriptions(key, scheme, now).await {
            Ok(body) => mapper::plan(&body),
            Err(error) => {
                tracing::debug!(%error, "Z.ai plan name unavailable");
                None
            }
        }
    }
}

#[async_trait]
impl Provider for ZaiProvider {
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
        let quota = self.quota(key, now).await?;
        Ok(LimitsSnapshot {
            identity: identity(key, quota.plan),
            windows: quota.windows,
            balances: Vec::new(),
            notices: Vec::new(),
            fetched_at: now,
            source: LimitsSource::Live,
        })
    }

    fn read_usage(&self, _: &Path, _: &mut LogCursors) -> Result<Vec<UsageEvent>, ProviderError> {
        Ok(Vec::new())
    }

    async fn validate_key(&self, key: &str) -> Result<AccountIdentity, ProviderError> {
        match self.quota(key, (self.clock)()).await {
            Ok(quota) => Ok(identity(key, quota.plan)),
            Err(ProviderError::NoSubscription { .. }) => Ok(identity(key, None)),
            Err(ProviderError::SignInExpired) => Err(ProviderError::Unsupported(format!(
                "Z.ai rejected this API key; check it at {CONSOLE_URL}"
            ))),
            Err(error) => Err(error),
        }
    }
}

fn identity(key: &str, plan: Option<String>) -> AccountIdentity {
    AccountIdentity {
        email: None,
        plan,
        stable_key: key_accounts::sha256_stable_key(key),
    }
}

fn no_plan() -> ProviderError {
    ProviderError::NoSubscription {
        detail: "No active GLM Coding Plan on this Z.ai account.".to_owned(),
    }
}

#[cfg(test)]
#[path = "provider_tests.rs"]
mod tests;
