use std::io::BufRead;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow, bail};
use headroom_core::account::{AccountIdentity, AccountRef, CredentialOwner};
use headroom_core::provider::Provider;
use headroom_core::secret::SecretString;
use headroom_providers::key_accounts;
use headroom_providers::secrets::SecretStore;

use super::announce::announce;
use super::cancel::{CANCELLED, Cancel};
use super::home::{create_home, discard_home};
use super::progress::ProgressEvent;
use crate::paths::Globals;

pub struct KeyTarget<'a> {
    pub provider: &'a dyn Provider,
    pub secrets: &'a SecretStore,
    pub root: &'a Path,
}

struct StoredKey {
    account: AccountRef,
    fresh: bool,
}

pub async fn add(
    globals: &Globals,
    target: &KeyTarget<'_>,
    input: impl BufRead,
    label: Option<&str>,
    events: &mut dyn FnMut(ProgressEvent) -> Result<()>,
    cancel: &Cancel,
) -> Result<(String, Option<String>)> {
    let key = read_key(input)?;
    let identity = tokio::select! {
        biased;
        () = cancel.cancelled() => bail!(CANCELLED),
        identity = validate(target.provider, &key) => identity?,
    };
    let stored = persist(target, &key, &identity).await?;
    let announced = async {
        events(ProgressEvent::Started {
            provider: stored.account.provider.clone(),
            home: stored.account.home.display().to_string(),
        })?;
        announce(globals, &stored.account, label).await
    };
    let result = tokio::select! {
        biased;
        () = cancel.cancelled() => Err(anyhow!(CANCELLED)),
        result = announced => result,
    };
    match result {
        Ok(labelled) => Ok((stored.account.id.0.clone(), labelled)),
        Err(error) => {
            forget(target.secrets, &stored).await;
            Err(error)
        }
    }
}

fn read_key(mut input: impl BufRead) -> Result<SecretString> {
    let mut line = String::new();
    input
        .read_line(&mut line)
        .context("could not read the API key from stdin")?;
    let key = line.trim();
    if key.is_empty() {
        bail!("no API key on stdin");
    }
    Ok(SecretString::new(key.to_owned()))
}

async fn validate(provider: &dyn Provider, key: &SecretString) -> Result<AccountIdentity> {
    provider
        .validate_key(key.expose())
        .await
        .map_err(|error| anyhow!("the API key was not accepted: {error}"))
}

async fn persist(
    target: &KeyTarget<'_>,
    key: &SecretString,
    identity: &AccountIdentity,
) -> Result<StoredKey> {
    let provider = target.provider;
    let id = identity.account_id(provider.id());
    let (home, fresh) = match existing_home(provider, &id.0).await {
        Some(home) => (home, false),
        None => (create_home(target.root, provider.id())?, true),
    };
    let account = AccountRef {
        id,
        provider: provider.id().clone(),
        home,
        owner: CredentialOwner::Headroom,
    };
    let stored = StoredKey { account, fresh };
    match save(target.secrets, &stored.account, key, identity).await {
        Ok(()) => Ok(stored),
        Err(error) => {
            forget(target.secrets, &stored).await;
            Err(error)
        }
    }
}

async fn forget(secrets: &SecretStore, stored: &StoredKey) {
    if !stored.fresh {
        return;
    }
    discard_home(&stored.account.home);
    if let Err(error) = secrets.delete(&stored.account.id).await {
        tracing::warn!(account = %stored.account.id, %error, "could not delete the stored key");
    }
}

async fn save(
    secrets: &SecretStore,
    account: &AccountRef,
    key: &SecretString,
    identity: &AccountIdentity,
) -> Result<()> {
    secrets
        .store(&account.provider, &account.id, key)
        .await
        .context("could not store the API key")?;
    key_accounts::save_record(&account.home, identity)?;
    Ok(())
}

async fn existing_home(provider: &dyn Provider, id: &str) -> Option<PathBuf> {
    let accounts = match provider.discover().await {
        Ok(accounts) => accounts,
        Err(error) => {
            tracing::debug!(%error, "no accounts to reuse");
            Vec::new()
        }
    };
    accounts
        .into_iter()
        .find(|account| account.id.0 == id && account.owner == CredentialOwner::Headroom)
        .map(|account| account.home)
}

#[cfg(test)]
#[path = "api_key_tests.rs"]
mod tests;
