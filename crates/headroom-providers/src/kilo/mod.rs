mod accounts;
mod auth;
mod client;
mod config;
mod mapper;
mod money;

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
use self::auth::KiloToken;
use self::client::{KiloClient, RawProfile};
use crate::key_accounts;

pub use self::config::{DEFAULT_API_BASE, KiloConfig};

pub const ID: ProviderId = ProviderId::from_static("kilo");
const DATA_SUBDIR: &str = "kilo";
const CONSOLE_URL: &str = "https://app.kilo.ai/profile";

pub static DESCRIPTOR: ProviderDescriptor = ProviderDescriptor {
    id: ID,
    display_name: "Kilo Code",
    add_account: &[
        AddAccountMethod::CliLogin(CliLogin {
            program: "kilo",
            args: &["auth", "login", "--provider", "kilo"],
            home_var: HomeVar::XdgBase {
                var: "XDG_DATA_HOME",
                subdir: DATA_SUBDIR,
            },
            credentials_file: auth::AUTH_FILE,
            needs_pty: true,
            scrub_env: &["KILO_API_URL", "KILO_AUTH_CONTENT"],
        }),
        AddAccountMethod::ApiKey(ApiKeyPrompt {
            label: "Kilo API key",
            console_url: CONSOLE_URL,
            hint: "Copy the API key from your Kilo profile; it shows your personal credit balance",
        }),
        AddAccountMethod::AutoDetect {
            reason: "Found when you sign in with `kilo auth login`",
        },
    ],
    multi_account: true,
    local_usage: false,
    links: ProviderLinks {
        status: Some("https://status.kilo.ai"),
        dashboard: Some("https://app.kilo.ai"),
        usage: Some("https://app.kilo.ai/usage"),
    },
};

pub type Clock = fn() -> Timestamp;

#[derive(Clone)]
pub struct KiloProvider {
    config: KiloConfig,
    client: KiloClient,
    secrets: Arc<dyn SecretReader>,
    clock: Clock,
}

impl KiloProvider {
    #[must_use]
    pub fn with_http(
        config: KiloConfig,
        http: reqwest::Client,
        secrets: Arc<dyn SecretReader>,
    ) -> KiloProvider {
        KiloProvider::with_clock(config, http, secrets, Timestamp::now)
    }

    #[must_use]
    pub fn with_clock(
        config: KiloConfig,
        http: reqwest::Client,
        secrets: Arc<dyn SecretReader>,
        clock: Clock,
    ) -> KiloProvider {
        KiloProvider {
            client: KiloClient::new(http, &config.api_base),
            config,
            secrets,
            clock,
        }
    }

    async fn token(&self, account: &AccountRef) -> Result<KiloToken, ProviderError> {
        match accounts::credential(account)? {
            Credential::Login(login) => Ok(login),
            Credential::Key => {
                let key = key_accounts::stored_key(self.secrets.as_ref(), account).await?;
                key_token(key, account)
            }
        }
    }
}

fn key_token(key: SecretString, account: &AccountRef) -> Result<KiloToken, ProviderError> {
    let token = KiloToken {
        token: key,
        organization_id: None,
    };
    if token.account_id() == account.id {
        Ok(token)
    } else {
        Err(ProviderError::AccountChanged(format!(
            "the Kilo API key stored for {} has changed",
            account.home.display()
        )))
    }
}

#[async_trait]
impl Provider for KiloProvider {
    fn descriptor(&self) -> &'static ProviderDescriptor {
        &DESCRIPTOR
    }

    async fn discover(&self) -> Result<Vec<AccountRef>, ProviderError> {
        accounts::discover(&self.config)
    }

    async fn account_at(&self, home: &Path) -> Result<Option<AccountRef>, ProviderError> {
        accounts::headroom_account_at(home)
    }

    async fn usage_homes(&self) -> Result<Vec<PathBuf>, ProviderError> {
        Ok(Vec::new())
    }

    async fn fetch_limits(&self, account: &AccountRef) -> Result<LimitsSnapshot, ProviderError> {
        let login = self.token(account).await?;
        let now = (self.clock)();
        let token = login.token.expose();
        let organization = login.organization_id.as_deref();
        let (balance, profile) = tokio::join!(
            self.client.balance(token, organization, now),
            self.client.profile(token, now)
        );
        let mapped = mapper::map(&balance?, organization.is_some());
        Ok(LimitsSnapshot {
            identity: AccountIdentity {
                email: profile_email(profile),
                ..login.identity()
            },
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
            .balance(key, None, (self.clock)())
            .await
            .map_err(rejected_key)?;
        let token = KiloToken {
            token: SecretString::new(key.to_owned()),
            organization_id: None,
        };
        Ok(token.identity())
    }
}

fn profile_email(profile: Result<RawProfile, ProviderError>) -> Option<String> {
    match profile {
        Ok(profile) => profile.user.email,
        Err(error) => {
            tracing::warn!(%error, "Kilo profile unavailable");
            None
        }
    }
}

fn rejected_key(error: ProviderError) -> ProviderError {
    match error {
        ProviderError::SignInExpired => ProviderError::Unsupported(format!(
            "Kilo rejected this API key; check it at {CONSOLE_URL}"
        )),
        other => other,
    }
}

#[cfg(test)]
#[path = "provider_tests.rs"]
mod tests;
