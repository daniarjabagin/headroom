mod file;
mod service;

use std::io;
use std::path::PathBuf;
use std::time::Duration;

use async_trait::async_trait;
use headroom_core::account::{AccountId, ProviderId};
use headroom_core::provider::ProviderError;
use headroom_core::secret::{SecretReader, SecretString};
use tokio::sync::Mutex;
use zbus::Connection;
use zbus::connection::Builder;

use file::FileSecrets;
use service::{Attributes, ServiceError};

const APPLICATION: &str = "io.github.headroom";
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
    File,
}

#[derive(Debug, thiserror::Error)]
pub enum SecretError {
    #[error("the keyring is locked; unlock it and try again")]
    Locked,
    #[error(
        "the Secret Service is unreachable and may still hold the API key; try again when it runs"
    )]
    Unreachable,
    #[error("Secret Service error: {0}")]
    Service(String),
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
pub async fn read_foreign(bus: &SecretBus, attributes: &[(&str, &str)]) -> ForeignSecret {
    let Some(conn) = connect(bus).await else {
        return ForeignSecret::Absent;
    };
    let attributes: Attributes = attributes.iter().copied().collect();
    match tokio::time::timeout(SERVICE_TIMEOUT, service::lookup(&conn, &attributes)).await {
        Ok(Ok(found)) => found,
        Ok(Err(error)) => {
            tracing::debug!(?error, "Secret Service lookup failed");
            ForeignSecret::Absent
        }
        Err(_) => {
            tracing::debug!("the Secret Service did not answer in time");
            ForeignSecret::Absent
        }
    }
}

/// API keys in the Secret Service, or in private files when no unlocked keyring is reachable.
pub struct SecretStore {
    bus: SecretBus,
    files: FileSecrets,
    conn: Mutex<Option<Connection>>,
}

impl SecretStore {
    #[must_use]
    pub fn new(bus: SecretBus, fallback_dir: PathBuf) -> SecretStore {
        SecretStore {
            bus,
            files: FileSecrets::new(fallback_dir),
            conn: Mutex::new(None),
        }
    }

    #[must_use]
    pub fn default_dir() -> Option<PathBuf> {
        dirs::data_dir().map(|data| data.join("headroom").join("secrets"))
    }

    pub async fn store(
        &self,
        provider: &ProviderId,
        account: &AccountId,
        secret: &SecretString,
    ) -> Result<SecretBackend, SecretError> {
        let attributes = attributes(Some(provider), account);
        let label = format!("Headroom API key for {account}");
        let stored = self
            .with_service(async |conn| service::store(conn, &attributes, &label, secret).await)
            .await;
        match stored {
            Ok(()) => {
                self.files.delete(account)?;
                Ok(SecretBackend::SecretService)
            }
            Err(ServiceError::Unavailable | ServiceError::Locked) => {
                self.files.write(account, secret)?;
                Ok(SecretBackend::File)
            }
            Err(ServiceError::Failed(error)) => Err(SecretError::Service(error.to_string())),
        }
    }

    pub async fn read(&self, account: &AccountId) -> Result<Option<SecretString>, SecretError> {
        let attributes = attributes(None, account);
        let found = self
            .with_service(async |conn| service::read(conn, &attributes).await)
            .await;
        match found {
            Ok(Some(secret)) => Ok(Some(secret)),
            Ok(None) | Err(ServiceError::Unavailable) => self.files.read(account),
            Err(error) => match self.files.read(account)? {
                Some(secret) => Ok(Some(secret)),
                None => Err(to_error(error)),
            },
        }
    }

    pub async fn delete(&self, account: &AccountId) -> Result<(), SecretError> {
        let attributes = attributes(None, account);
        let from_service = self
            .with_service(async |conn| service::delete(conn, &attributes).await)
            .await;
        match from_service {
            Ok(()) => self.files.delete(account).map(drop),
            Err(ServiceError::Unavailable) => self.delete_without_service(account),
            Err(error) => Err(to_error(error)),
        }
    }

    fn delete_without_service(&self, account: &AccountId) -> Result<(), SecretError> {
        let had_file = self.files.delete(account)?;
        if had_file || self.bus == SecretBus::Disabled {
            Ok(())
        } else {
            Err(SecretError::Unreachable)
        }
    }

    async fn with_service<T>(
        &self,
        operation: impl AsyncFnOnce(&Connection) -> Result<T, ServiceError>,
    ) -> Result<T, ServiceError> {
        let Some(conn) = self.connection().await else {
            return Err(ServiceError::Unavailable);
        };
        tokio::time::timeout(SERVICE_TIMEOUT, operation(&conn))
            .await
            .unwrap_or_else(|_| {
                tracing::debug!("the Secret Service did not answer in time");
                Err(ServiceError::Unavailable)
            })
    }

    async fn connection(&self) -> Option<Connection> {
        let mut cached = self.conn.lock().await;
        if let Some(conn) = cached.as_ref() {
            return Some(conn.clone());
        }
        let conn = connect(&self.bus).await?;
        *cached = Some(conn.clone());
        Some(conn)
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

fn attributes<'a>(provider: Option<&'a ProviderId>, account: &'a AccountId) -> Attributes<'a> {
    let mut attributes = Attributes::from([
        ("application", APPLICATION),
        ("account", account.0.as_str()),
    ]);
    if let Some(provider) = provider {
        attributes.insert("provider", provider.as_str());
    }
    attributes
}

async fn connect(bus: &SecretBus) -> Option<Connection> {
    let builder = match bus {
        SecretBus::Session => Builder::session(),
        SecretBus::Address(address) => Builder::address(address.as_str()),
        SecretBus::Disabled => return None,
    };
    let connected = tokio::time::timeout(SERVICE_TIMEOUT, async { builder?.build().await }).await;
    match connected {
        Ok(Ok(conn)) => Some(conn),
        Ok(Err(error)) => {
            tracing::debug!(%error, "no session bus for the Secret Service");
            None
        }
        Err(_) => None,
    }
}

fn to_error(error: ServiceError) -> SecretError {
    match error {
        ServiceError::Locked => SecretError::Locked,
        ServiceError::Unavailable => {
            SecretError::Service("the Secret Service is unavailable".into())
        }
        ServiceError::Failed(error) => SecretError::Service(error.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attributes_name_the_application_account_and_provider() {
        let provider = ProviderId::from_static("tool");
        let account = AccountId("tool:0123456789ab".into());
        let full = attributes(Some(&provider), &account);
        assert_eq!(
            full,
            Attributes::from([
                ("application", "io.github.headroom"),
                ("provider", "tool"),
                ("account", "tool:0123456789ab"),
            ])
        );
        assert_eq!(attributes(None, &account).len(), 2);
    }

    #[tokio::test]
    async fn without_a_secret_service_keys_use_the_file_fallback() {
        let root = tempfile::tempdir().unwrap();
        let store = SecretStore::new(SecretBus::Disabled, root.path().join("secrets"));
        let provider = ProviderId::from_static("tool");
        let account = AccountId("tool:0123456789ab".into());
        let key = SecretString::new("sk-test".into());
        assert_eq!(store.read(&account).await.unwrap(), None);
        let backend = store.store(&provider, &account, &key).await.unwrap();
        assert_eq!(backend, SecretBackend::File);
        assert_eq!(store.read_secret(&account).await.unwrap(), Some(key));
        store.delete(&account).await.unwrap();
        assert_eq!(store.read(&account).await.unwrap(), None);
    }

    #[tokio::test]
    async fn an_unreachable_bus_falls_back_to_files() {
        let root = tempfile::tempdir().unwrap();
        let address = format!("unix:path={}", root.path().join("no-bus").display());
        let store = SecretStore::new(SecretBus::Address(address), root.path().join("secrets"));
        let account = AccountId("tool:0123456789ab".into());
        let key = SecretString::new("sk-test".into());
        let provider = ProviderId::from_static("tool");
        assert_eq!(
            store.store(&provider, &account, &key).await.unwrap(),
            SecretBackend::File
        );
        assert_eq!(store.read(&account).await.unwrap(), Some(key));
    }

    #[tokio::test]
    async fn foreign_items_are_absent_without_a_reachable_bus() {
        let attributes = [("service", "tool")];
        let disabled = read_foreign(&SecretBus::Disabled, &attributes).await;
        assert_eq!(disabled, ForeignSecret::Absent);
        let root = tempfile::tempdir().unwrap();
        let address = format!("unix:path={}", root.path().join("no-bus").display());
        let unreachable = read_foreign(&SecretBus::Address(address), &attributes).await;
        assert_eq!(unreachable, ForeignSecret::Absent);
    }
}
