use headroom_core::account::{AccountId, ProviderId};
use headroom_core::secret::SecretString;

#[cfg(target_os = "linux")]
use super::service::Attributes;
#[cfg(target_os = "linux")]
use super::session::SecretService;
use super::{APPLICATION, SecretBackend, SecretError};
use crate::keychain::{GenericPassword, KeychainError, Security};

#[derive(Debug)]
pub(super) enum BackendError {
    Unavailable,
    Locked,
    Failed(SecretError),
}

pub(super) enum Backend {
    #[cfg(target_os = "linux")]
    SecretService(SecretService),
    Keychain(Security),
    None,
}

impl Backend {
    pub(super) fn kind(&self) -> SecretBackend {
        match self {
            #[cfg(target_os = "linux")]
            Backend::SecretService(_) => SecretBackend::SecretService,
            Backend::Keychain(_) => SecretBackend::Keychain,
            Backend::None => SecretBackend::File,
        }
    }

    pub(super) async fn store(
        &self,
        provider: &ProviderId,
        account: &AccountId,
        secret: &SecretString,
    ) -> Result<(), BackendError> {
        match self {
            #[cfg(target_os = "linux")]
            Backend::SecretService(service) => {
                let label = format!("Headroom API key for {account}");
                let attributes = attributes(Some(provider), account);
                service.store(&attributes, &label, secret).await
            }
            Backend::Keychain(security) => {
                let label = format!("Headroom {provider} API key");
                security
                    .store(item(account), &label, secret)
                    .await
                    .map_err(keychain_error)
            }
            Backend::None => Err(BackendError::Unavailable),
        }
    }

    pub(super) async fn read(
        &self,
        account: &AccountId,
    ) -> Result<Option<SecretString>, BackendError> {
        match self {
            #[cfg(target_os = "linux")]
            Backend::SecretService(service) => service.read(&attributes(None, account)).await,
            Backend::Keychain(security) => {
                security.find(item(account)).await.map_err(keychain_error)
            }
            Backend::None => Err(BackendError::Unavailable),
        }
    }

    pub(super) async fn delete(&self, account: &AccountId) -> Result<(), BackendError> {
        match self {
            #[cfg(target_os = "linux")]
            Backend::SecretService(service) => service.delete(&attributes(None, account)).await,
            Backend::Keychain(security) => security
                .delete(item(account))
                .await
                .map(drop)
                .map_err(keychain_error),
            Backend::None => Err(BackendError::Unavailable),
        }
    }
}

#[cfg(target_os = "linux")]
pub(super) fn attributes<'a>(
    provider: Option<&'a ProviderId>,
    account: &'a AccountId,
) -> Attributes<'a> {
    let mut attributes = Attributes::from([
        ("application", APPLICATION),
        ("account", account.0.as_str()),
    ]);
    if let Some(provider) = provider {
        attributes.insert("provider", provider.as_str());
    }
    attributes
}

pub(super) fn item(account: &AccountId) -> GenericPassword<'_> {
    GenericPassword {
        service: APPLICATION,
        account: Some(account.0.as_str()),
    }
}

fn keychain_error(error: KeychainError) -> BackendError {
    match error {
        error if error.is_unavailable() => BackendError::Unavailable,
        KeychainError::Denied(_) => BackendError::Locked,
        error => BackendError::Failed(SecretError::Keychain(error)),
    }
}
