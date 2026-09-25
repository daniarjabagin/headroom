use std::fs::{self, File, OpenOptions, TryLockError};
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;

use headroom_core::provider::ProviderError;
use jiff::Timestamp;
use serde_json::Value;

use super::auth::{AUTH_FILE, Credentials, credentials_from, read_auth_file};
use super::oauth::{RawTokens, TokenClient};
use crate::fsio::write_private;

const LOCK_FILE: &str = "auth.json.lock";
const PRIVATE_FILE: u32 = 0o600;

pub(super) async fn refresh(
    client: &TokenClient,
    home: &Path,
    stale: &Credentials,
    now: Timestamp,
) -> Result<Credentials, ProviderError> {
    let client = client.clone();
    let home = home.to_path_buf();
    let stale = stale.clone();
    let detached = tokio::spawn(async move { refresh_locked(&client, &home, &stale, now).await });
    detached
        .await
        .map_err(|_| ProviderError::LocalData("the Codex sign-in refresh was interrupted".into()))?
}

async fn refresh_locked(
    client: &TokenClient,
    home: &Path,
    stale: &Credentials,
    now: Timestamp,
) -> Result<Credentials, ProviderError> {
    let _lock = lock_auth(home)?;
    let file = read_auth_file(home)?;
    let current = credentials_from(&file.document, file.modified_at)?;
    if current.access_token != stale.access_token && current.ensure_fresh(now).is_ok() {
        return Ok(current);
    }
    let refresh_token = refresh_token(&file.document).ok_or(ProviderError::SignInExpired)?;
    let tokens = client.refresh(&refresh_token, now).await?;
    let patched = patch(file.document, &tokens, now)?;
    let refreshed = credentials_from(&patched, current.signed_in_at)?;
    if let Err(error) = store(home, &file.bytes, &patched) {
        tracing::error!(home = %home.display(), %error, "refreshed codex sign-in not saved");
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
            "another process is refreshing this Codex sign-in".into(),
        )),
        Err(TryLockError::Error(error)) => Err(local_error(&path, &error.to_string())),
    }
}

fn refresh_token(document: &Value) -> Option<String> {
    document
        .get("tokens")?
        .get("refresh_token")?
        .as_str()
        .map(str::trim)
        .filter(|token| !token.is_empty())
        .map(str::to_owned)
}

pub(super) fn patch(
    mut document: Value,
    tokens: &RawTokens,
    now: Timestamp,
) -> Result<Value, ProviderError> {
    let access = non_empty(tokens.access.as_deref()).ok_or_else(|| {
        ProviderError::InvalidResponse("token refresh returned no access token".into())
    })?;
    let root = document
        .as_object_mut()
        .ok_or_else(|| ProviderError::LocalData("auth.json is not an object".into()))?;
    let stored = root
        .get_mut("tokens")
        .and_then(Value::as_object_mut)
        .ok_or_else(|| ProviderError::LocalData("auth.json lost its tokens".into()))?;
    stored.insert("access_token".into(), access.into());
    for (name, value) in [("id_token", &tokens.id), ("refresh_token", &tokens.refresh)] {
        if let Some(value) = non_empty(value.as_deref()) {
            stored.insert(name.into(), value.into());
        }
    }
    root.insert("last_refresh".into(), now.to_string().into());
    Ok(document)
}

fn non_empty(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|text| !text.is_empty())
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
