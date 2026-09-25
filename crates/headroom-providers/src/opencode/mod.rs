mod auth;
mod client;
mod config;
mod mapper;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use headroom_core::account::{AccountIdentity, AccountRef, CredentialOwner, ProviderId};
use headroom_core::cursor::LogCursors;
use headroom_core::descriptor::{AddAccountMethod, ApiKeyPrompt, ProviderDescriptor};
use headroom_core::event::UsageEvent;
use headroom_core::provider::{Provider, ProviderError};
use headroom_core::quota::{LimitsSnapshot, LimitsSource};
use headroom_core::secret::{SecretReader, SecretString};
use jiff::Timestamp;

use self::auth::{identity_for_key, read_cli_key};
use self::client::UsageClient;
use crate::key_accounts;

pub use self::config::{DEFAULT_API_BASE, OpenCodeConfig};

pub const ID: ProviderId = ProviderId::from_static("opencode");
const CONSOLE_URL: &str = "https://opencode.ai/auth";

pub static DESCRIPTOR: ProviderDescriptor = ProviderDescriptor {
    id: ID,
    display_name: "OpenCode",
    add_account: &[
        AddAccountMethod::ApiKey(ApiKeyPrompt {
            label: "API key",
            console_url: CONSOLE_URL,
            hint: "The key's workspace needs an OpenCode Go subscription",
        }),
        AddAccountMethod::AutoDetect {
            reason: "sign in to OpenCode Go with `opencode auth login`",
        },
    ],
    multi_account: true,
    local_usage: false,
};

pub type Clock = fn() -> Timestamp;

pub struct OpenCodeProvider {
    config: OpenCodeConfig,
    client: UsageClient,
    secrets: Arc<dyn SecretReader>,
    clock: Clock,
}

impl OpenCodeProvider {
    #[must_use]
    pub fn with_http(
        config: OpenCodeConfig,
        http: reqwest::Client,
        secrets: Arc<dyn SecretReader>,
    ) -> OpenCodeProvider {
        OpenCodeProvider::with_clock(config, http, secrets, Timestamp::now)
    }

    #[must_use]
    pub fn with_clock(
        config: OpenCodeConfig,
        http: reqwest::Client,
        secrets: Arc<dyn SecretReader>,
        clock: Clock,
    ) -> OpenCodeProvider {
        OpenCodeProvider {
            client: UsageClient::new(http, &config.api_base),
            config,
            secrets,
            clock,
        }
    }

    fn cli_account(&self) -> Result<Option<AccountRef>, ProviderError> {
        let Some(key) = read_cli_key(&self.config.data_dir)? else {
            return Ok(None);
        };
        Ok(Some(AccountRef {
            id: identity_for_key(key.expose()).account_id(&ID),
            provider: ID,
            home: self.config.data_dir.clone(),
            owner: CredentialOwner::Cli,
        }))
    }

    async fn account_key(&self, account: &AccountRef) -> Result<SecretString, ProviderError> {
        let key = match account.owner {
            CredentialOwner::Headroom => self.secrets.read_secret(&account.id).await?,
            CredentialOwner::Cli => read_cli_key(&account.home)?,
        }
        .ok_or(ProviderError::NotSignedIn)?;
        if identity_for_key(key.expose()).account_id(&ID) == account.id {
            Ok(key)
        } else {
            Err(ProviderError::AccountChanged(format!(
                "the OpenCode Go key at {} has changed",
                account.home.display()
            )))
        }
    }
}

#[async_trait]
impl Provider for OpenCodeProvider {
    fn descriptor(&self) -> &'static ProviderDescriptor {
        &DESCRIPTOR
    }

    async fn discover(&self) -> Result<Vec<AccountRef>, ProviderError> {
        let mut accounts = key_accounts::discover(&self.config.accounts_dir, &ID)?;
        match self.cli_account() {
            Ok(cli) => accounts.extend(cli),
            Err(error) if accounts.is_empty() => return Err(error),
            Err(error) => tracing::warn!(%error, "skipping the OpenCode login"),
        }
        Ok(accounts)
    }

    async fn usage_homes(&self) -> Result<Vec<PathBuf>, ProviderError> {
        Ok(Vec::new())
    }

    async fn fetch_limits(&self, account: &AccountRef) -> Result<LimitsSnapshot, ProviderError> {
        let key = self.account_key(account).await?;
        let now = (self.clock)();
        let raw = self.client.fetch(key.expose(), now).await?;
        Ok(LimitsSnapshot {
            identity: identity_for_key(key.expose()),
            windows: mapper::map_usage(&raw),
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
        self.client
            .fetch(key, (self.clock)())
            .await
            .map_err(rejected_key)?;
        Ok(identity_for_key(key))
    }
}

fn rejected_key(error: ProviderError) -> ProviderError {
    match error {
        ProviderError::SignInExpired | ProviderError::NotSignedIn => ProviderError::Unsupported(
            format!("OpenCode rejected this API key; check it at {CONSOLE_URL}"),
        ),
        other => other,
    }
}

#[cfg(test)]
#[path = "provider_tests.rs"]
mod tests;
