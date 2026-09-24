use std::fs::{self, File, OpenOptions, TryLockError};
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;

use headroom_core::provider::ProviderError;
use jiff::{SignedDuration, Timestamp};
use serde_json::Value;

use super::auth::{AUTH_FILE, Credentials, credentials_from, read_auth_file};
use super::client::{GrokClient, RawTokens};
use crate::fsio::write_private;

const LOCK_FILE: &str = "auth.json.lock";
const PRIVATE_FILE: u32 = 0o600;

pub(super) async fn refresh(
    client: &GrokClient,
    issuer: &str,
    home: &Path,
    stale: &Credentials,
    now: Timestamp,
) -> Result<Credentials, ProviderError> {
    let client = client.clone();
    let issuer = issuer.to_owned();
    let home = home.to_path_buf();
    let stale = stale.clone();
    let detached =
        tokio::spawn(async move { refresh_locked(&client, &issuer, &home, &stale, now).await });
    detached
        .await
        .map_err(|_| ProviderError::LocalData("the Grok sign-in refresh was interrupted".into()))?
}

async fn refresh_locked(
    client: &GrokClient,
    issuer: &str,
    home: &Path,
    stale: &Credentials,
    now: Timestamp,
) -> Result<Credentials, ProviderError> {
    let _lock = lock_auth(home)?;
    let file = read_auth_file(home)?;
    let current = credentials_from(&file.document)?;
    if current.access_token != stale.access_token && !current.expires_soon(now) {
        return Ok(current);
    }
    let (client_id, refresh_token) = refresh_inputs(&current, issuer)?;
    let tokens = client.refresh(issuer, client_id, refresh_token).await?;
    let patched = patch(file.document, &current.entry, &tokens, now)?;
    let refreshed = credentials_from(&patched)?;
    if let Err(error) = store(home, &file.bytes, &patched) {
        tracing::error!(home = %home.display(), %error, "refreshed grok sign-in not saved");
    }
    Ok(refreshed)
}

fn lock_auth(home: &Path) -> Result<File, ProviderError> {
    let path = home.join(LOCK_FILE);
    let file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .mode(PRIVATE_FILE)
        .open(&path)
        .map_err(|error| local_error(&path, &error.to_string()))?;
    match file.try_lock() {
        Ok(()) => Ok(file),
        Err(TryLockError::WouldBlock) => Err(ProviderError::LocalData(
            "another process is refreshing this Grok sign-in".into(),
        )),
        Err(TryLockError::Error(error)) => Err(local_error(&path, &error.to_string())),
    }
}

fn refresh_inputs<'a>(
    credentials: &'a Credentials,
    issuer: &str,
) -> Result<(&'a str, &'a str), ProviderError> {
    let same_issuer = credentials
        .issuer
        .as_deref()
        .is_some_and(|known| known.trim_end_matches('/') == issuer.trim_end_matches('/'));
    match (&credentials.client_id, &credentials.refresh_token) {
        (Some(client_id), Some(refresh_token)) if same_issuer => Ok((client_id, refresh_token)),
        _ => Err(ProviderError::SignInExpired),
    }
}

pub(super) fn patch(
    mut document: Value,
    entry: &str,
    tokens: &RawTokens,
    now: Timestamp,
) -> Result<Value, ProviderError> {
    let access = tokens.access_token.trim();
    if access.is_empty() {
        return Err(ProviderError::InvalidResponse(
            "token refresh returned no access token".into(),
        ));
    }
    let fields = document
        .get_mut(entry)
        .and_then(Value::as_object_mut)
        .ok_or_else(|| ProviderError::LocalData("auth.json lost its Grok entry".into()))?;
    fields.insert("key".into(), access.into());
    for (name, value) in [
        ("refresh_token", &tokens.refresh_token),
        ("id_token", &tokens.id_token),
    ] {
        if let Some(value) = value.as_deref().map(str::trim).filter(|v| !v.is_empty()) {
            fields.insert(name.into(), value.into());
        }
    }
    fields.remove("expires");
    match tokens.expires_in.filter(|seconds| *seconds > 0) {
        Some(seconds) => {
            let expiry = now
                .checked_add(SignedDuration::from_secs(seconds))
                .map_err(|_| {
                    ProviderError::InvalidResponse("token lifetime out of range".into())
                })?;
            fields.insert("expires_at".into(), expiry.to_string().into());
        }
        None => {
            fields.remove("expires_at");
        }
    }
    Ok(document)
}

fn store(home: &Path, read: &[u8], document: &Value) -> Result<(), ProviderError> {
    let path = home.join(AUTH_FILE);
    let on_disk = fs::read(&path).map_err(|error| local_error(&path, &error.to_string()))?;
    if on_disk != read {
        return Err(local_error(&path, "changed during the refresh"));
    }
    let bytes = serde_json::to_vec_pretty(document)
        .map_err(|error| local_error(&path, &error.to_string()))?;
    write_private(&path, &bytes).map_err(|error| local_error(&path, &error.to_string()))
}

fn local_error(path: &Path, problem: &str) -> ProviderError {
    ProviderError::LocalData(format!("{}: {problem}", path.display()))
}

#[cfg(test)]
#[path = "refresh_tests.rs"]
mod tests;
