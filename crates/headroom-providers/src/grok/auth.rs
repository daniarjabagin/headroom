use std::fmt;
use std::fs;
use std::io;
use std::path::Path;

use headroom_core::account::AccountIdentity;
use headroom_core::provider::ProviderError;
use jiff::{SignedDuration, Timestamp};
use serde_json::{Map, Value};

pub(super) const AUTH_FILE: &str = "auth.json";
const REFRESH_AHEAD: SignedDuration = SignedDuration::from_mins(5);

#[derive(Clone, PartialEq, Eq)]
pub(super) struct Credentials {
    pub entry: String,
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub client_id: Option<String>,
    pub issuer: Option<String>,
    pub expires_at: Option<Timestamp>,
    pub identity: AccountIdentity,
}

impl Credentials {
    pub(super) fn is_expired(&self, now: Timestamp) -> bool {
        self.expires_at.is_some_and(|expiry| expiry <= now)
    }

    pub(super) fn expires_soon(&self, now: Timestamp) -> bool {
        self.expires_at
            .is_some_and(|expiry| expiry.duration_since(now) <= REFRESH_AHEAD)
    }
}

impl fmt::Debug for Credentials {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Credentials")
            .field("entry", &self.entry)
            .field("access_token", &"<redacted>")
            .field(
                "refresh_token",
                &self.refresh_token.as_ref().map(|_| "<redacted>"),
            )
            .field("client_id", &self.client_id)
            .field("issuer", &self.issuer)
            .field("expires_at", &self.expires_at)
            .field("identity", &self.identity)
            .finish()
    }
}

pub(super) struct AuthFile {
    pub bytes: Vec<u8>,
    pub document: Value,
}

pub(super) fn load_credentials(home: &Path) -> Result<Credentials, ProviderError> {
    credentials_from(&read_auth_file(home)?.document)
}

pub(super) fn read_auth_file(home: &Path) -> Result<AuthFile, ProviderError> {
    let path = home.join(AUTH_FILE);
    let bytes = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return Err(ProviderError::NotSignedIn);
        }
        Err(error) => {
            return Err(ProviderError::LocalData(format!(
                "cannot read {}: {error}",
                path.display()
            )));
        }
    };
    let document = serde_json::from_slice::<Value>(&bytes)
        .ok()
        .filter(Value::is_object)
        .ok_or_else(|| ProviderError::LocalData(format!("{} is not valid JSON", path.display())))?;
    Ok(AuthFile { bytes, document })
}

pub(super) fn credentials_from(document: &Value) -> Result<Credentials, ProviderError> {
    let (entry, fields) = document
        .as_object()
        .and_then(signed_in_entry)
        .ok_or(ProviderError::NotSignedIn)?;
    let Some(access_token) = text(fields, "key") else {
        return Err(ProviderError::NotSignedIn);
    };
    Ok(Credentials {
        access_token,
        refresh_token: text(fields, "refresh_token").or_else(|| text(fields, "refresh")),
        client_id: text(fields, "oidc_client_id").or_else(|| name_part(entry, 1)),
        issuer: text(fields, "oidc_issuer").or_else(|| name_part(entry, 0)),
        expires_at: expiry(fields)?,
        identity: identity(fields)?,
        entry: entry.to_owned(),
    })
}

fn signed_in_entry(entries: &Map<String, Value>) -> Option<(&str, &Map<String, Value>)> {
    entries.iter().find_map(|(name, value)| {
        let fields = value.as_object()?;
        text(fields, "key").map(|_| (name.as_str(), fields))
    })
}

fn identity(fields: &Map<String, Value>) -> Result<AccountIdentity, ProviderError> {
    let user = text(fields, "user_id")
        .ok_or_else(|| ProviderError::LocalData("auth.json has no Grok user id".into()))?;
    let team = text(fields, "team_id").unwrap_or_default();
    Ok(AccountIdentity {
        email: text(fields, "email"),
        plan: None,
        stable_key: format!("{user}/{team}"),
    })
}

fn expiry(fields: &Map<String, Value>) -> Result<Option<Timestamp>, ProviderError> {
    let Some(raw) = text(fields, "expires_at").or_else(|| text(fields, "expires")) else {
        return Ok(None);
    };
    raw.parse::<Timestamp>()
        .map(Some)
        .map_err(|_| ProviderError::LocalData("auth.json has an unreadable expiry".into()))
}

fn name_part(entry: &str, index: usize) -> Option<String> {
    let (issuer, client) = entry.split_once("::")?;
    [issuer, client]
        .get(index)
        .map(|part| part.trim())
        .filter(|part| !part.is_empty())
        .map(str::to_owned)
}

fn text(fields: &Map<String, Value>, key: &str) -> Option<String> {
    fields
        .get(key)?
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

#[cfg(test)]
#[path = "auth_tests.rs"]
mod tests;
