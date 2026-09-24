mod backend;
mod file;
#[cfg(target_os = "linux")]
mod service;
#[cfg(target_os = "linux")]
mod session;

use std::io;
use std::path::PathBuf;
#[cfg(target_os = "linux")]
use std::time::Duration;

use async_trait::async_trait;
use headroom_core::account::{AccountId, ProviderId};
use headroom_core::provider::ProviderError;
use headroom_core::secret::{SecretReader, SecretString};

use backend::{Backend, BackendError};
use file::FileSecrets;

use crate::keychain::{KeychainError, Security};
use crate::paths::{HeadroomDirs, Os};

const APPLICATION: &str = "io.github.headroom";
#[cfg(target_os = "linux")]
const SERVICE_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SecretBus {
    Session,
    Address(String),
    Disabled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecretBackend {
    SecretService,
    Keychain,
    File,
}

#[derive(Debug, thiserror::Error)]
pub enum SecretError {
    #[error("the keyring is locked; unlock it and try again")]
    Locked,
    #[error("the keyring is unreachable and may still hold the API key; try again when it runs")]
    Unreachable,
    #[error("Secret Service error: {0}")]
    Service(String),
    #[error("Keychain error: {0}")]
    Keychain(KeychainError),
    #[error("cannot {action} {}: {kind}", path.display())]
    Io {
        action: &'static str,
        path: PathBuf,
        kind: io::ErrorKind,
    },
    #[error("{0:?} is not a valid account id")]
    InvalidAccount(String),
    #[error("the stored secret is not valid UTF-8")]
    NotUtf8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ForeignSecret {
    Found(SecretString),
    Locked,
    Absent,
}

/// Another application's Secret Service item, read without ever unlocking or prompting.
#[cfg(target_os = "linux")]
pub async fn read_foreign(bus: &SecretBus, attributes: &[(&str, &str)]) -> ForeignSecret {
    session::read_foreign(bus, attributes).await
}

/// Another application's Secret Service item; there is no Secret Service off Linux.
#[cfg(not(target_os = "linux"))]
#[allow(
    clippy::unused_async,
    reason = "keeps the signature of the Linux Secret Service lookup"
)]
pub async fn read_foreign(_bus: &SecretBus, _attributes: &[(&str, &str)]) -> ForeignSecret {
    ForeignSecret::Absent
}

/// API keys in the system keyring, or in private files when no unlocked keyring is reachable.
pub struct SecretStore {
    backend: Backend,
    files: FileSecrets,
}

impl SecretStore {
    /// The Secret Service on Linux, the Keychain on macOS; `Disabled` keeps keys in files only.
    #[must_use]
    pub fn new(bus: SecretBus, fallback_dir: PathBuf) -> SecretStore {
        let backend = match (Os::current(), bus) {
            (_, SecretBus::Disabled) => Backend::None,
            (Os::MacOs, _) => Backend::Keychain(Security::system()),
            #[cfg(target_os = "linux")]
            (Os::Linux, bus) => Backend::SecretService(session::SecretService::new(bus)),
            #[cfg(not(target_os = "linux"))]
            (Os::Linux, _) => Backend::None,
        };
        SecretStore::with_backend(backend, fallback_dir)
    }

    #[must_use]
    pub fn with_keychain(security: Security, fallback_dir: PathBuf) -> SecretStore {
        SecretStore::with_backend(Backend::Keychain(security), fallback_dir)
    }

    fn with_backend(backend: Backend, fallback_dir: PathBuf) -> SecretStore {
        SecretStore {
            backend,
            files: FileSecrets::new(fallback_dir),
        }
    }

    #[must_use]
    pub fn default_dir() -> Option<PathBuf> {
        HeadroomDirs::from_process().map(|dirs| dirs.secrets())
    }

    pub async fn store(
        &self,
        provider: &ProviderId,
        account: &AccountId,
        secret: &SecretString,
    ) -> Result<SecretBackend, SecretError> {
        match self.backend.store(provider, account, secret).await {
            Ok(()) => {
                self.files.delete(account)?;
                Ok(self.backend.kind())
            }
            Err(BackendError::Unavailable | BackendError::Locked) => {
                self.files.write(account, secret)?;
                Ok(SecretBackend::File)
            }
            Err(BackendError::Failed(error)) => Err(error),
        }
    }

    pub async fn read(&self, account: &AccountId) -> Result<Option<SecretString>, SecretError> {
        match self.backend.read(account).await {
            Ok(Some(secret)) => Ok(Some(secret)),
            Ok(None) | Err(BackendError::Unavailable) => self.files.read(account),
            Err(error) => match self.files.read(account)? {
                Some(secret) => Ok(Some(secret)),
                None => Err(to_error(error)),
            },
        }
    }

    pub async fn delete(&self, account: &AccountId) -> Result<(), SecretError> {
        match self.backend.delete(account).await {
            Ok(()) => self.files.delete(account).map(drop),
            Err(BackendError::Unavailable) => self.delete_without_backend(account),
            Err(error) => Err(to_error(error)),
        }
    }

    fn delete_without_backend(&self, account: &AccountId) -> Result<(), SecretError> {
        let had_file = self.files.delete(account)?;
        if had_file || matches!(self.backend, Backend::None) {
            Ok(())
        } else {
            Err(SecretError::Unreachable)
        }
    }
}

#[async_trait]
impl SecretReader for SecretStore {
    async fn read_secret(
        &self,
        account: &AccountId,
    ) -> Result<Option<SecretString>, ProviderError> {
        self.read(account)
            .await
            .map_err(|error| ProviderError::LocalData(error.to_string()))
    }
}

fn to_error(error: BackendError) -> SecretError {
    match error {
        BackendError::Locked => SecretError::Locked,
        BackendError::Unavailable => SecretError::Unreachable,
        BackendError::Failed(error) => error,
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
