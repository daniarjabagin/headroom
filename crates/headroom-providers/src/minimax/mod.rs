mod client;
mod config;
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
use headroom_core::quota::{LimitsSnapshot, LimitsSource, QuotaWindow};
use headroom_core::secret::SecretReader;
use jiff::Timestamp;

use self::client::MiniMaxClient;
use crate::key_accounts;

pub use self::config::{GLOBAL_API_BASE, MiniMaxConfig};

pub const ID: ProviderId = ProviderId::from_static("minimax");
const CONSOLE_URL: &str = "https://platform.minimax.io/user-center/payment/token-plan";

pub static DESCRIPTOR: ProviderDescriptor = ProviderDescriptor {
    id: ID,
    display_name: "MiniMax",
    add_account: &[AddAccountMethod::ApiKey(ApiKeyPrompt {
        label: "MiniMax Token Plan key",
        console_url: CONSOLE_URL,
        hint: "Use the Token Plan subscription key from platform.minimax.io; \
               pay-as-you-go keys have no plan limits",
    })],
    multi_account: true,
    local_usage: false,
    min_poll_interval: None,
    links: ProviderLinks {
        status: Some("https://status.minimax.io"),
        dashboard: Some("https://platform.minimax.io/console/plan"),
        usage: Some("https://platform.minimax.io/console/plan"),
    },
};

const PLAN: &str = "Token Plan";

pub type Clock = fn() -> Timestamp;

#[derive(Clone)]
pub struct MiniMaxProvider {
    config: MiniMaxConfig,
    client: MiniMaxClient,
    secrets: Arc<dyn SecretReader>,
    clock: Clock,
}

impl MiniMaxProvider {
    #[must_use]
    pub fn with_http(
        config: MiniMaxConfig,
        http: reqwest::Client,
        secrets: Arc<dyn SecretReader>,
    ) -> MiniMaxProvider {
        MiniMaxProvider::with_clock(config, http, secrets, Timestamp::now)
    }

    #[must_use]
    pub fn with_clock(
        config: MiniMaxConfig,
        http: reqwest::Client,
        secrets: Arc<dyn SecretReader>,
        clock: Clock,
    ) -> MiniMaxProvider {
        MiniMaxProvider {
            client: MiniMaxClient::new(http, &config.api_base),
            config,
            secrets,
            clock,
        }
    }

    async fn windows(&self, key: &str, now: Timestamp) -> Result<Vec<QuotaWindow>, ProviderError> {
        let raw = self.client.remains(key, now).await?;
        mapper::map_remains(&raw)
    }

    fn stored_identity(account: &AccountRef) -> Result<AccountIdentity, ProviderError> {
        let identity =
            key_accounts::load_record(&account.home)?.ok_or(ProviderError::NotSignedIn)?;
        if identity.account_id(&ID) == account.id {
            Ok(identity)
        } else {
            Err(ProviderError::AccountChanged(format!(
                "the MiniMax account stored at {} has changed",
                account.home.display()
            )))
        }
    }
}

fn key_identity(key: &str) -> AccountIdentity {
    AccountIdentity {
        email: None,
        plan: Some(PLAN.to_owned()),
        stable_key: key_accounts::fingerprint_stable_key(key),
    }
}

#[async_trait]
impl Provider for MiniMaxProvider {
    fn descriptor(&self) -> &'static ProviderDescriptor {
        &DESCRIPTOR
    }

    async fn discover(&self) -> Result<Vec<AccountRef>, ProviderError> {
        key_accounts::discover(&self.config.accounts_dir, &ID)
    }

    async fn usage_homes(&self) -> Result<Vec<PathBuf>, ProviderError> {
        Ok(Vec::new())
    }

    async fn fetch_limits(&self, account: &AccountRef) -> Result<LimitsSnapshot, ProviderError> {
        let identity = MiniMaxProvider::stored_identity(account)?;
        let key = key_accounts::stored_key(self.secrets.as_ref(), account).await?;
        let now = (self.clock)();
        Ok(LimitsSnapshot {
            windows: self.windows(key.expose(), now).await?,
            identity,
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
        self.windows(key, (self.clock)())
            .await
            .map_err(rejected_key)?;
        Ok(key_identity(key))
    }
}

fn rejected_key(error: ProviderError) -> ProviderError {
    match error {
        ProviderError::SignInExpired | ProviderError::NotSignedIn => ProviderError::Unsupported(
            format!("MiniMax rejected this API key; check it at {CONSOLE_URL}"),
        ),
        other => other,
    }
}

#[cfg(test)]
#[path = "provider_tests.rs"]
mod tests;
