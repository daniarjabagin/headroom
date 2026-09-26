mod accounts;
mod auth;
mod client;
mod config;
mod credentials;
mod mapper;
mod raw;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use headroom_core::account::{AccountIdentity, AccountRef, ProviderId};
use headroom_core::cursor::LogCursors;
use headroom_core::descriptor::{
    AddAccountMethod, ApiKeyPrompt, CliLogin, HomeVar, ProviderDescriptor, ProviderLinks,
};
use headroom_core::event::UsageEvent;
use headroom_core::provider::{Provider, ProviderError};
use headroom_core::quota::{LimitsSnapshot, LimitsSource};
use headroom_core::secret::{SecretReader, SecretString};
use jiff::Timestamp;

use self::accounts::Credential;
use self::client::KimiClient;
use self::mapper::MappedUsage;
use crate::key_accounts;

pub use self::config::{DEFAULT_API_BASE, DEFAULT_OAUTH_HOST, KimiConfig};

pub const ID: ProviderId = ProviderId::from_static("kimi");
const CONSOLE_URL: &str = "https://www.kimi.com/code/console";

pub static DESCRIPTOR: ProviderDescriptor = ProviderDescriptor {
    id: ID,
    display_name: "Kimi Code",
    add_account: &[
        AddAccountMethod::ApiKey(ApiKeyPrompt {
            label: "Kimi Code API key",
            console_url: CONSOLE_URL,
            hint: "Create it in the Kimi Code console; Moonshot platform keys do not work",
        }),
        AddAccountMethod::CliLogin(CliLogin {
            program: "kimi",
            args: &["login"],
            home_var: HomeVar::Direct("KIMI_SHARE_DIR"),
            credentials_file: credentials::CREDENTIALS_FILE,
            default_dir: ".kimi",
            needs_pty: false,
            scrub_env: &[],
        }),
    ],
    multi_account: true,
    local_usage: false,
    links: ProviderLinks {
        status: Some("https://status.moonshot.cn"),
        dashboard: Some("https://www.kimi.com/code/console"),
        usage: Some("https://www.kimi.com/code/console"),
    },
};

pub type Clock = fn() -> Timestamp;

#[derive(Clone)]
pub struct KimiProvider {
    config: KimiConfig,
    client: KimiClient,
    secrets: Arc<dyn SecretReader>,
    clock: Clock,
}

impl KimiProvider {
    #[must_use]
    pub fn with_http(
        config: KimiConfig,
        http: reqwest::Client,
        secrets: Arc<dyn SecretReader>,
    ) -> KimiProvider {
        KimiProvider::with_clock(config, http, secrets, Timestamp::now)
    }

    #[must_use]
    pub fn with_clock(
        config: KimiConfig,
        http: reqwest::Client,
        secrets: Arc<dyn SecretReader>,
        clock: Clock,
    ) -> KimiProvider {
        KimiProvider {
            client: KimiClient::new(http, &config.api_base, &config.oauth_host),
            config,
            secrets,
            clock,
        }
    }

    async fn bearer(
        &self,
        account: &AccountRef,
        now: Timestamp,
    ) -> Result<(SecretString, AccountIdentity), ProviderError> {
        match accounts::credential(&account.home)? {
            Credential::Key(identity) => {
                check_account(account, &identity)?;
                let key = key_accounts::stored_key(self.secrets.as_ref(), account).await?;
                Ok((key, identity))
            }
            Credential::OAuth(identity) => {
                let token = auth::oauth_token(&self.client, account, now).await?;
                Ok((token, identity))
            }
        }
    }

    async fn usage(&self, bearer: &str, now: Timestamp) -> Result<MappedUsage, ProviderError> {
        let raw = self.client.usages(bearer, now).await?;
        mapper::map_usage(&raw)
    }
}

fn check_account(account: &AccountRef, identity: &AccountIdentity) -> Result<(), ProviderError> {
    if identity.account_id(&ID) == account.id {
        Ok(())
    } else {
        Err(ProviderError::AccountChanged(format!(
            "the Kimi account stored at {} has changed",
            account.home.display()
        )))
    }
}

#[async_trait]
impl Provider for KimiProvider {
    fn descriptor(&self) -> &'static ProviderDescriptor {
        &DESCRIPTOR
    }

    async fn discover(&self) -> Result<Vec<AccountRef>, ProviderError> {
        accounts::discover(&self.config)
    }

    async fn usage_homes(&self) -> Result<Vec<PathBuf>, ProviderError> {
        Ok(Vec::new())
    }

    async fn fetch_limits(&self, account: &AccountRef) -> Result<LimitsSnapshot, ProviderError> {
        let now = (self.clock)();
        let (bearer, identity) = self.bearer(account, now).await?;
        let mapped = self.usage(bearer.expose(), now).await?;
        Ok(LimitsSnapshot {
            identity: AccountIdentity {
                plan: mapped.plan.or(identity.plan),
                ..identity
            },
            windows: mapped.windows,
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
        let mapped = self
            .usage(key, (self.clock)())
            .await
            .map_err(rejected_key)?;
        Ok(accounts::key_identity(key, mapped.plan))
    }
}

fn rejected_key(error: ProviderError) -> ProviderError {
    match error {
        ProviderError::SignInExpired | ProviderError::NotSignedIn => ProviderError::Unsupported(
            format!("Kimi Code rejected this API key; check it at {CONSOLE_URL}"),
        ),
        other => other,
    }
}

#[cfg(test)]
#[path = "provider_tests.rs"]
mod tests;
