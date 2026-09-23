mod client;
mod key;
mod locate;
mod mapper;
mod money;
mod raw;

use std::path::{Path, PathBuf};

use async_trait::async_trait;
use headroom_core::account::{AccountIdentity, AccountRef, CredentialOwner, ProviderId};
use headroom_core::cursor::LogCursors;
use headroom_core::descriptor::{AddAccountMethod, ProviderDescriptor};
use headroom_core::event::UsageEvent;
use headroom_core::provider::{Provider, ProviderError};
use headroom_core::quota::{LimitsSnapshot, LimitsSource};
use jiff::Timestamp;

use client::CloudClient;
use key::SigningKey;
use locate::{KeyFile, locate};

pub use client::DEFAULT_API_BASE;
pub use locate::KeyPaths;

pub const ID: ProviderId = ProviderId::from_static("ollama");

pub static DESCRIPTOR: ProviderDescriptor = ProviderDescriptor {
    id: ID,
    display_name: "Ollama Cloud",
    add_account: &[AddAccountMethod::AutoDetect {
        reason: "Headroom signs requests with the key Ollama keeps in ~/.ollama/id_ed25519; \
                 run `ollama signin` to link it to your ollama.com account",
    }],
    multi_account: false,
    local_usage: false,
};

pub type Clock = fn() -> Timestamp;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OllamaConfig {
    pub keys: KeyPaths,
    pub api_base: String,
}

impl OllamaConfig {
    #[must_use]
    pub fn for_home(home: &Path) -> OllamaConfig {
        OllamaConfig {
            keys: KeyPaths::from_home(home),
            api_base: DEFAULT_API_BASE.to_owned(),
        }
    }

    pub fn from_env() -> Result<OllamaConfig, ProviderError> {
        let home = dirs::home_dir()
            .ok_or_else(|| ProviderError::LocalData("home directory not found".to_owned()))?;
        Ok(OllamaConfig::for_home(&home))
    }
}

#[derive(Debug)]
pub struct OllamaProvider {
    keys: KeyPaths,
    client: CloudClient,
    clock: Clock,
}

impl OllamaProvider {
    #[must_use]
    pub fn with_http(config: OllamaConfig, http: reqwest::Client) -> OllamaProvider {
        OllamaProvider::with_clock(config, http, Timestamp::now)
    }

    #[must_use]
    pub fn with_clock(config: OllamaConfig, http: reqwest::Client, clock: Clock) -> OllamaProvider {
        OllamaProvider {
            client: CloudClient::new(http, &config.api_base),
            keys: config.keys,
            clock,
        }
    }

    async fn fetch_signed(&self, path: &Path, pem: &str) -> Result<LimitsSnapshot, ProviderError> {
        let key = SigningKey::parse(pem).map_err(|_| {
            ProviderError::LocalData(format!(
                "{} is not a usable Ollama signing key",
                path.display()
            ))
        })?;
        let now = (self.clock)();
        let usage = self.client.usage(&key, now).await?;
        let me = self
            .client
            .me(&key, now)
            .await
            .inspect_err(|error| tracing::warn!(%error, "ollama plan lookup failed"))
            .ok();
        Ok(mapper::map_snapshot(
            &usage,
            me.as_ref(),
            stable_key(path),
            now,
        ))
    }

    async fn is_unlinked(&self, pem: &str) -> bool {
        let Ok(key) = SigningKey::parse(pem) else {
            return false;
        };
        let answer = self.client.me(&key, (self.clock)()).await;
        matches!(answer, Err(ProviderError::SignInExpired))
    }

    fn unreadable(&self, path: &Path) -> LimitsSnapshot {
        let text = if path == self.keys.system {
            format!(
                "Ollama runs as a system service and its signing key ({}) is not readable \
                 by your user. Run Ollama as your own user and sign in with `ollama signin` \
                 to see Cloud usage.",
                path.display()
            )
        } else {
            format!(
                "Headroom cannot read the Ollama signing key at {}. Check that your user \
                 owns it.",
                path.display()
            )
        };
        LimitsSnapshot {
            identity: identity(path),
            windows: Vec::new(),
            balances: Vec::new(),
            notices: vec![mapper::notice(&text)],
            fetched_at: (self.clock)(),
            source: LimitsSource::Live,
        }
    }
}

#[async_trait]
impl Provider for OllamaProvider {
    fn descriptor(&self) -> &'static ProviderDescriptor {
        &DESCRIPTOR
    }

    async fn discover(&self) -> Result<Vec<AccountRef>, ProviderError> {
        let located = locate(&self.keys);
        let path = located.path().ok_or(ProviderError::NotSignedIn)?;
        if let KeyFile::Found { pem, .. } = &located
            && self.is_unlinked(pem).await
        {
            return Ok(Vec::new());
        }
        Ok(vec![account_ref(path)])
    }

    async fn usage_homes(&self) -> Result<Vec<PathBuf>, ProviderError> {
        Ok(Vec::new())
    }

    async fn fetch_limits(&self, _account: &AccountRef) -> Result<LimitsSnapshot, ProviderError> {
        match locate(&self.keys) {
            KeyFile::Found { path, pem } => self.fetch_signed(&path, &pem).await,
            KeyFile::Unreadable { path } => Ok(self.unreadable(&path)),
            KeyFile::Missing => Err(ProviderError::NotSignedIn),
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

fn stable_key(key_path: &Path) -> String {
    key_path.display().to_string()
}

fn identity(key_path: &Path) -> AccountIdentity {
    AccountIdentity {
        email: None,
        plan: None,
        stable_key: stable_key(key_path),
    }
}

fn account_ref(key_path: &Path) -> AccountRef {
    AccountRef {
        id: identity(key_path).account_id(&ID),
        provider: ID,
        home: key_path.parent().unwrap_or(key_path).to_path_buf(),
        owner: CredentialOwner::Cli,
    }
}

#[cfg(test)]
#[path = "provider_tests.rs"]
mod tests;
