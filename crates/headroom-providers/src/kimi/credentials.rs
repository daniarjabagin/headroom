use std::fs;
use std::io;
use std::path::Path;

use headroom_core::provider::ProviderError;
use headroom_core::secret::SecretString;
use jiff::{SignedDuration, Timestamp};
use serde::Deserialize;
use serde_json::{Map, Number, Value};

use crate::fsio::write_private;

pub(super) const CREDENTIALS_FILE: &str = "credentials/kimi-code.json";

#[derive(Clone)]
pub(super) struct OAuthTokens {
    pub(super) access: SecretString,
    pub(super) refresh: SecretString,
    pub(super) expires_at: Option<Timestamp>,
    fields: Map<String, Value>,
}

#[derive(Clone, Deserialize)]
pub(super) struct RefreshedTokens {
    access_token: String,
    refresh_token: String,
    expires_in: Number,
    scope: Option<String>,
    token_type: Option<String>,
}

pub(super) fn has_credentials(home: &Path) -> bool {
    home.join(CREDENTIALS_FILE).is_file()
}

pub(super) fn load(home: &Path) -> Result<OAuthTokens, ProviderError> {
    let path = home.join(CREDENTIALS_FILE);
    let text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return Err(ProviderError::NotSignedIn);
        }
        Err(error) => return Err(local_error("read", &path, &error)),
    };
    parse(&text).map_err(|problem| {
        ProviderError::LocalData(format!("cannot parse {}: {problem}", path.display()))
    })
}

fn parse(text: &str) -> Result<OAuthTokens, String> {
    let fields: Map<String, Value> = serde_json::from_str(text).map_err(|e| e.to_string())?;
    let token = |name: &str| {
        fields
            .get(name)
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .map(|value| SecretString::new(value.to_owned()))
            .ok_or_else(|| format!("{name} is missing"))
    };
    Ok(OAuthTokens {
        access: token("access_token")?,
        refresh: token("refresh_token")?,
        expires_at: fields.get("expires_at").and_then(epoch_seconds),
        fields,
    })
}

fn epoch_seconds(value: &Value) -> Option<Timestamp> {
    let seconds = SignedDuration::try_from_secs_f64(value.as_f64()?).ok()?;
    Timestamp::from_duration(seconds).ok()
}

impl OAuthTokens {
    pub(super) fn valid_for(&self, now: Timestamp, margin: SignedDuration) -> bool {
        self.expires_at
            .is_none_or(|expires_at| expires_at.duration_since(now) > margin)
    }
}

impl RefreshedTokens {
    pub(super) fn access(&self) -> SecretString {
        SecretString::new(self.access_token.clone())
    }
}

pub(super) fn save_refreshed(
    home: &Path,
    previous: &OAuthTokens,
    refreshed: &RefreshedTokens,
    now: Timestamp,
) -> Result<(), ProviderError> {
    let mut fields = previous.fields.clone();
    let expires_at = expiry(refreshed, now)?;
    fields.insert("access_token".into(), refreshed.access_token.clone().into());
    fields.insert(
        "refresh_token".into(),
        refreshed.refresh_token.clone().into(),
    );
    fields.insert("expires_at".into(), expires_at.as_second().into());
    fields.insert(
        "expires_in".into(),
        Value::Number(refreshed.expires_in.clone()),
    );
    for (name, value) in [
        ("scope", &refreshed.scope),
        ("token_type", &refreshed.token_type),
    ] {
        if let Some(value) = value {
            fields.insert(name.into(), value.clone().into());
        }
    }
    let path = home.join(CREDENTIALS_FILE);
    let bytes = serde_json::to_vec(&fields)
        .map_err(|error| ProviderError::LocalData(format!("cannot encode tokens: {error}")))?;
    write_private(&path, &bytes).map_err(|error| local_error("write", &path, &error))
}

fn expiry(refreshed: &RefreshedTokens, now: Timestamp) -> Result<Timestamp, ProviderError> {
    let invalid =
        || ProviderError::InvalidResponse("Kimi returned an unusable token expiry".into());
    let lifetime = refreshed
        .expires_in
        .as_f64()
        .and_then(|seconds| SignedDuration::try_from_secs_f64(seconds).ok())
        .filter(SignedDuration::is_positive)
        .ok_or_else(invalid)?;
    now.checked_add(lifetime).map_err(|_| invalid())
}

fn local_error(action: &str, path: &Path, error: &io::Error) -> ProviderError {
    ProviderError::LocalData(format!(
        "cannot {action} {}: {}",
        path.display(),
        error.kind()
    ))
}

#[cfg(test)]
#[path = "credentials_tests.rs"]
mod tests;
