use std::fs::{self, File, OpenOptions, TryLockError};
use std::io;
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;

use headroom_core::provider::ProviderError;
use jiff::{SignedDuration, Timestamp};
use serde_json::{Map, Value};

use super::auth::{CREDENTIALS_FILE, Credentials, parse_credentials};
use super::oauth::{CLIENT_ID, RawTokens, RefreshRequest, TokenClient};
use crate::fsio::write_private;

const LOCK_FILE: &str = ".credentials.json.lock";
const OAUTH_ENTRY: &str = "claudeAiOauth";
const PRIVATE_FILE: u32 = 0o600;

pub(super) async fn refresh(
    client: &TokenClient,
    home: &Path,
    stale: &Credentials,
    now: Timestamp,
) -> Result<Credentials, ProviderError> {
    let client = client.clone();
    let home = home.to_path_buf();
    let stale_token = stale.token_secret().to_owned();
    let detached =
        tokio::spawn(async move { refresh_locked(&client, &home, &stale_token, now).await });
    detached.await.map_err(|_| {
        ProviderError::LocalData("the Claude sign-in refresh was interrupted".into())
    })?
}

async fn refresh_locked(
    client: &TokenClient,
    home: &Path,
    stale_token: &str,
    now: Timestamp,
) -> Result<Credentials, ProviderError> {
    let _lock = lock_credentials(home)?;
    let path = home.join(CREDENTIALS_FILE);
    let bytes = read(&path)?;
    let text = String::from_utf8_lossy(&bytes);
    let current = parse_credentials(&text)?;
    if current.token_secret() != stale_token && current.usable_token(now).is_ok() {
        return Ok(current);
    }
    let document: Value =
        serde_json::from_str(&text).map_err(|_| local_error(&path, "is not valid JSON"))?;
    let oauth = oauth_entry(&document).ok_or(ProviderError::NotSignedIn)?;
    let refresh_token = text_field(oauth, "refreshToken").ok_or(ProviderError::SignInExpired)?;
    let request = RefreshRequest {
        grant_type: "refresh_token",
        refresh_token: &refresh_token,
        client_id: &text_field(oauth, "clientId").unwrap_or_else(|| CLIENT_ID.to_owned()),
        scope: scopes(oauth),
    };
    let tokens = client.refresh(&request, now).await?;
    let patched = patch(document, &tokens, now)?;
    let refreshed = parse_credentials(&patched.to_string())?;
    match store(&path, &bytes, &patched, now) {
        Ok(Stored::Written) => Ok(refreshed),
        Ok(Stored::Adopted(current)) => Ok(current),
        Err(error) => {
            tracing::error!(home = %home.display(), %error, "refreshed claude sign-in not saved");
            Ok(refreshed)
        }
    }
}

enum Stored {
    Written,
    Adopted(Credentials),
}

fn lock_credentials(home: &Path) -> Result<File, ProviderError> {
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
            "another process is refreshing this Claude sign-in".into(),
        )),
        Err(TryLockError::Error(error)) => Err(local_error(&path, &error.to_string())),
    }
}

fn read(path: &Path) -> Result<Vec<u8>, ProviderError> {
    fs::read(path).map_err(|error| match error.kind() {
        io::ErrorKind::NotFound => ProviderError::NotSignedIn,
        _ => local_error(path, &error.to_string()),
    })
}

fn oauth_entry(document: &Value) -> Option<&Map<String, Value>> {
    document.get(OAUTH_ENTRY)?.as_object()
}

fn scopes(oauth: &Map<String, Value>) -> Option<String> {
    let scopes: Vec<&str> = oauth
        .get("scopes")?
        .as_array()?
        .iter()
        .filter_map(Value::as_str)
        .collect();
    (!scopes.is_empty()).then(|| scopes.join(" "))
}

pub(super) fn patch(
    mut document: Value,
    tokens: &RawTokens,
    now: Timestamp,
) -> Result<Value, ProviderError> {
    let access = non_empty(tokens.access_token.as_deref()).ok_or_else(|| {
        ProviderError::InvalidResponse("token refresh returned no access token".into())
    })?;
    let oauth = document
        .get_mut(OAUTH_ENTRY)
        .and_then(Value::as_object_mut)
        .ok_or_else(|| ProviderError::LocalData("the Claude sign-in lost its tokens".into()))?;
    oauth.insert("accessToken".into(), access.into());
    if let Some(refresh) = non_empty(tokens.refresh_token.as_deref()) {
        oauth.insert("refreshToken".into(), refresh.into());
    }
    match expiry_millis(tokens.expires_in, now)? {
        Some(millis) => oauth.insert("expiresAt".into(), millis.into()),
        None => oauth.remove("expiresAt"),
    };
    if let Some(scope) = non_empty(tokens.scope.as_deref()) {
        let granted: Vec<Value> = scope.split_whitespace().map(Value::from).collect();
        oauth.insert("scopes".into(), granted.into());
    }
    Ok(document)
}

fn expiry_millis(expires_in: Option<i64>, now: Timestamp) -> Result<Option<i64>, ProviderError> {
    let Some(seconds) = expires_in.filter(|seconds| *seconds > 0) else {
        return Ok(None);
    };
    now.checked_add(SignedDuration::from_secs(seconds))
        .map(|expiry| Some(expiry.as_millisecond()))
        .map_err(|_| ProviderError::InvalidResponse("token lifetime out of range".into()))
}

fn text_field(fields: &Map<String, Value>, key: &str) -> Option<String> {
    non_empty(fields.get(key)?.as_str()).map(str::to_owned)
}

fn non_empty(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|text| !text.is_empty())
}

fn store(
    path: &Path,
    read: &[u8],
    document: &Value,
    now: Timestamp,
) -> Result<Stored, ProviderError> {
    let on_disk = fs::read(path).map_err(|error| local_error(path, &error.to_string()))?;
    if on_disk != read {
        if let Some(current) = unexpired(&on_disk, now) {
            return Ok(Stored::Adopted(current));
        }
        tracing::warn!(path = %path.display(), "claude sign-in changed during the refresh, keeping the rotated tokens");
    }
    let bytes = serde_json::to_vec_pretty(document)
        .map_err(|error| local_error(path, &error.to_string()))?;
    write_private(path, &bytes).map_err(|error| local_error(path, &error.to_string()))?;
    Ok(Stored::Written)
}

fn unexpired(bytes: &[u8], now: Timestamp) -> Option<Credentials> {
    let text = std::str::from_utf8(bytes).ok()?;
    parse_credentials(text)
        .ok()
        .filter(|credentials| !credentials.is_expired(now))
}

fn local_error(path: &Path, problem: &str) -> ProviderError {
    ProviderError::LocalData(format!("{}: {problem}", path.display()))
}

#[cfg(test)]
#[path = "refresh_tests.rs"]
mod tests;
