mod client;
mod discovery;
mod keyring;
mod mapper;
mod process;
mod raw;
#[cfg(test)]
mod test_support;
mod token;

use std::path::{Path, PathBuf};

use async_trait::async_trait;
use headroom_core::account::{AccountIdentity, AccountRef, CredentialOwner, ProviderId};
use headroom_core::cursor::LogCursors;
use headroom_core::descriptor::{AddAccountMethod, ProviderDescriptor};
use headroom_core::event::UsageEvent;
use headroom_core::pace::Tone;
use headroom_core::provider::{Provider, ProviderError};
use headroom_core::quota::{LimitsSnapshot, LimitsSource, Notice, QuotaWindow};
use jiff::Timestamp;

use client::{CloudClient, LanguageServerClient};
use discovery::LanguageServer;
use keyring::KeyringItem;
use raw::{RawSummaryEnvelope, RawUserStatusEnvelope};
use token::AccessToken;

use crate::secrets::SecretBus;

pub use client::DEFAULT_CLOUD_BASES;

pub const ID: ProviderId = ProviderId::from_static("antigravity");

pub static DESCRIPTOR: ProviderDescriptor = ProviderDescriptor {
    id: ID,
    display_name: "Antigravity",
    add_account: &[AddAccountMethod::AutoDetect {
        reason: "Headroom reads quota from the running Antigravity app or `agy`, \
                 or from the sign-in Antigravity keeps in the keyring",
    }],
    multi_account: false,
    local_usage: false,
};

const NOT_FOUND_TEXT: &str = "Headroom cannot find an Antigravity sign-in. Start Antigravity or \
                              run `agy`, then refresh. agy 1.0.1 and later keep their sign-in \
                              where Headroom cannot read it while agy is closed.";
const LOCKED_TEXT: &str =
    "The keyring that holds the Antigravity sign-in is locked. Unlock it or start Antigravity.";
const NO_POOLS_TEXT: &str = "Antigravity reports no quota pools for this account.";
const NO_TOKEN_TEXT: &str = "the Antigravity keyring item holds no usable access token";

pub type Clock = fn() -> Timestamp;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AntigravityConfig {
    pub gemini_dir: PathBuf,
    pub proc_root: PathBuf,
    pub cloud_bases: Vec<String>,
    pub secret_bus: SecretBus,
}

impl AntigravityConfig {
    #[must_use]
    pub fn for_home(home: &Path) -> AntigravityConfig {
        AntigravityConfig {
            gemini_dir: home.join(".gemini"),
            proc_root: PathBuf::from("/proc"),
            cloud_bases: DEFAULT_CLOUD_BASES.map(str::to_owned).to_vec(),
            secret_bus: SecretBus::Session,
        }
    }

    pub fn from_env() -> Result<AntigravityConfig, ProviderError> {
        let home = dirs::home_dir()
            .ok_or_else(|| ProviderError::LocalData("home directory not found".to_owned()))?;
        Ok(AntigravityConfig::for_home(&home))
    }
}

#[derive(Debug)]
pub struct AntigravityProvider {
    config: AntigravityConfig,
    language_server: LanguageServerClient,
    cloud: CloudClient,
    clock: Clock,
}

impl AntigravityProvider {
    pub fn with_http(
        config: AntigravityConfig,
        http: reqwest::Client,
    ) -> Result<AntigravityProvider, ProviderError> {
        AntigravityProvider::with_clock(config, http, Timestamp::now)
    }

    pub fn with_clock(
        config: AntigravityConfig,
        http: reqwest::Client,
        clock: Clock,
    ) -> Result<AntigravityProvider, ProviderError> {
        Ok(AntigravityProvider {
            language_server: LanguageServerClient::new()?,
            cloud: CloudClient::new(http, &config.cloud_bases),
            config,
            clock,
        })
    }

    async fn language_servers(&self) -> Vec<LanguageServer> {
        let proc_root = self.config.proc_root.clone();
        tokio::task::spawn_blocking(move || discovery::language_servers(&proc_root))
            .await
            .unwrap_or_default()
    }

    async fn query_language_servers(&self) -> Option<LimitsSnapshot> {
        for server in self.language_servers().await {
            for endpoint in server.endpoints() {
                if let Some(snapshot) = self.query_endpoint(&endpoint, &server.csrf).await {
                    return Some(snapshot);
                }
            }
        }
        None
    }

    async fn query_endpoint(&self, endpoint: &str, csrf: &str) -> Option<LimitsSnapshot> {
        let ls = &self.language_server;
        let summary: RawSummaryEnvelope =
            ls.call(endpoint, csrf, "RetrieveUserQuotaSummary").await?;
        let windows = mapper::windows(summary.groups()?);
        let status: Option<RawUserStatusEnvelope> = ls.call(endpoint, csrf, "GetUserStatus").await;
        let plan = status.as_ref().and_then(mapper::language_server_plan);
        Some(self.snapshot(windows, plan))
    }

    async fn query_keyring(&self) -> Result<LimitsSnapshot, ProviderError> {
        match keyring::read(&self.config.secret_bus).await {
            KeyringItem::Found(raw) => {
                let token = token::parse(&raw)
                    .ok_or_else(|| ProviderError::LocalData(NO_TOKEN_TEXT.to_owned()))?;
                self.query_cloud(&token).await
            }
            KeyringItem::Locked => Ok(self.notice_only(LOCKED_TEXT)),
            KeyringItem::Absent => Ok(self.notice_only(NOT_FOUND_TEXT)),
        }
    }

    async fn query_cloud(&self, token: &AccessToken) -> Result<LimitsSnapshot, ProviderError> {
        if token.is_expired((self.clock)()) {
            return Err(ProviderError::SignInExpired);
        }
        let summary = self.cloud.quota_summary(&token.secret).await?;
        let groups = summary.groups().ok_or_else(|| {
            ProviderError::InvalidResponse("Cloud Code quota summary has no groups".to_owned())
        })?;
        let windows = mapper::windows(groups);
        let plan = self
            .cloud
            .code_assist(&token.secret)
            .await
            .inspect_err(|error| tracing::debug!(%error, "antigravity plan lookup failed"))
            .ok()
            .and_then(|assist| mapper::cloud_plan(&assist));
        Ok(self.snapshot(windows, plan))
    }

    fn snapshot(&self, windows: Vec<QuotaWindow>, plan: Option<String>) -> LimitsSnapshot {
        let notices = if windows.is_empty() {
            vec![warning(NO_POOLS_TEXT)]
        } else {
            Vec::new()
        };
        LimitsSnapshot {
            identity: AccountIdentity {
                email: None,
                plan,
                stable_key: self.stable_key(),
            },
            windows,
            balances: Vec::new(),
            notices,
            fetched_at: (self.clock)(),
            source: LimitsSource::Live,
        }
    }

    fn notice_only(&self, text: &str) -> LimitsSnapshot {
        LimitsSnapshot {
            notices: vec![warning(text)],
            ..self.snapshot(Vec::new(), None)
        }
    }

    fn stable_key(&self) -> String {
        format!("antigravity:{}", self.config.gemini_dir.display())
    }

    fn account(&self) -> AccountRef {
        let identity = AccountIdentity {
            email: None,
            plan: None,
            stable_key: self.stable_key(),
        };
        AccountRef {
            id: identity.account_id(&ID),
            provider: ID,
            home: self.config.gemini_dir.clone(),
            owner: CredentialOwner::Cli,
        }
    }
}

#[async_trait]
impl Provider for AntigravityProvider {
    fn descriptor(&self) -> &'static ProviderDescriptor {
        &DESCRIPTOR
    }

    async fn discover(&self) -> Result<Vec<AccountRef>, ProviderError> {
        if discovery::installed(&self.config.gemini_dir)
            || !self.language_servers().await.is_empty()
        {
            Ok(vec![self.account()])
        } else {
            Err(ProviderError::NotSignedIn)
        }
    }

    async fn usage_homes(&self) -> Result<Vec<PathBuf>, ProviderError> {
        Ok(Vec::new())
    }

    async fn fetch_limits(&self, _account: &AccountRef) -> Result<LimitsSnapshot, ProviderError> {
        match self.query_language_servers().await {
            Some(snapshot) => Ok(snapshot),
            None => self.query_keyring().await,
        }
    }

    fn read_usage(
        &self,
        _home: &Path,
        _cursors: &mut LogCursors,
    ) -> Result<Vec<UsageEvent>, ProviderError> {
        Ok(Vec::new())
    }
}

fn warning(text: &str) -> Notice {
    Notice {
        tone: Tone::Warning,
        text: text.to_owned(),
    }
}

#[cfg(test)]
#[path = "provider_tests.rs"]
mod tests;
