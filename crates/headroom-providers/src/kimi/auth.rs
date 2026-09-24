use std::fs::{File, OpenOptions, TryLockError};
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;

use headroom_core::account::{AccountRef, CredentialOwner};
use headroom_core::provider::ProviderError;
use headroom_core::secret::SecretString;
use jiff::{SignedDuration, Timestamp};

use super::client::KimiClient;
use super::credentials;

const REFRESH_MARGIN: SignedDuration = SignedDuration::from_mins(5);
const LOCK_FILE: &str = "credentials/kimi-code.json.lock";
const PRIVATE_FILE: u32 = 0o600;

pub(super) async fn oauth_token(
    client: &KimiClient,
    account: &AccountRef,
    now: Timestamp,
) -> Result<SecretString, ProviderError> {
    let tokens = credentials::load(&account.home)?;
    if tokens.valid_for(now, REFRESH_MARGIN) {
        return Ok(tokens.access);
    }
    if account.owner == CredentialOwner::Cli {
        return if tokens.valid_for(now, SignedDuration::ZERO) {
            Ok(tokens.access)
        } else {
            Err(ProviderError::SignInExpired)
        };
    }
    refresh(client, &account.home, now).await
}

async fn refresh(
    client: &KimiClient,
    home: &Path,
    now: Timestamp,
) -> Result<SecretString, ProviderError> {
    let client = client.clone();
    let home = home.to_path_buf();
    let detached = tokio::spawn(async move { refresh_locked(&client, &home, now).await });
    detached
        .await
        .map_err(|_| ProviderError::LocalData("the Kimi sign-in refresh was interrupted".into()))?
}

async fn refresh_locked(
    client: &KimiClient,
    home: &Path,
    now: Timestamp,
) -> Result<SecretString, ProviderError> {
    let _lock = lock_credentials(home)?;
    let stored = credentials::read(home)?;
    if stored.tokens.valid_for(now, REFRESH_MARGIN) {
        return Ok(stored.tokens.access);
    }
    let refreshed = client.refresh(stored.tokens.refresh.expose(), now).await?;
    let bytes = credentials::refreshed_document(&stored.tokens, &refreshed, now)?;
    if let Err(error) = credentials::replace_unchanged(home, &stored.bytes, &bytes) {
        tracing::error!(home = %home.display(), %error, "refreshed kimi sign-in not saved");
    }
    Ok(refreshed.access())
}

fn lock_credentials(home: &Path) -> Result<File, ProviderError> {
    let path = home.join(LOCK_FILE);
    let local_error =
        |problem: String| ProviderError::LocalData(format!("{}: {problem}", path.display()));
    let file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .mode(PRIVATE_FILE)
        .open(&path)
        .map_err(|error| local_error(error.to_string()))?;
    match file.try_lock() {
        Ok(()) => Ok(file),
        Err(TryLockError::WouldBlock) => Err(ProviderError::LocalData(
            "another process is refreshing this Kimi sign-in".into(),
        )),
        Err(TryLockError::Error(error)) => Err(local_error(error.to_string())),
    }
}

#[cfg(test)]
#[path = "auth_tests.rs"]
mod tests;
