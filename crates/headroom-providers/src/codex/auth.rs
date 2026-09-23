use std::fmt;
use std::fs;
use std::io;
use std::path::Path;

use headroom_core::account::AccountIdentity;
use headroom_core::provider::ProviderError;
use jiff::Timestamp;
use serde_json::Value;

use super::jwt::{expires_at, id_claims};
use super::labels::plan_label;

const AUTH_FILE: &str = "auth.json";

#[derive(Clone, PartialEq, Eq)]
pub(super) struct Credentials {
    pub access_token: String,
    pub account_id: Option<String>,
    pub identity: AccountIdentity,
    pub expires_at: Option<Timestamp>,
}

impl Credentials {
    pub(super) fn ensure_fresh(&self, now: Timestamp) -> Result<(), ProviderError> {
        match self.expires_at {
            Some(expiry) if expiry <= now => Err(ProviderError::SignInExpired),
            _ => Ok(()),
        }
    }
}

impl fmt::Debug for Credentials {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Credentials")
            .field("access_token", &"<redacted>")
            .field("account_id", &self.account_id)
            .field("identity", &self.identity)
            .field("expires_at", &self.expires_at)
            .finish()
    }
}

pub(super) fn load_credentials(home: &Path) -> Result<Credentials, ProviderError> {
    let document = read_auth_document(home)?;
    credentials_from(&document)
}

fn read_auth_document(home: &Path) -> Result<Value, ProviderError> {
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
    parse_auth_document(&bytes)
        .ok_or_else(|| ProviderError::LocalData(format!("{} is not valid JSON", path.display())))
}

fn parse_auth_document(bytes: &[u8]) -> Option<Value> {
    serde_json::from_slice::<Value>(bytes)
        .ok()
        .filter(Value::is_object)
        .or_else(|| parse_hex_document(bytes))
}

fn parse_hex_document(bytes: &[u8]) -> Option<Value> {
    let text = std::str::from_utf8(bytes).ok()?.trim();
    let decoded = hex::decode(text).ok()?;
    serde_json::from_slice::<Value>(&decoded)
        .ok()
        .filter(Value::is_object)
}

fn credentials_from(document: &Value) -> Result<Credentials, ProviderError> {
    let tokens = document.get("tokens");
    let token_text = |key: &str| tokens.and_then(|tokens| non_empty(tokens, key));
    let Some(access_token) = token_text("access_token") else {
        return Err(missing_token_error(document));
    };
    let claims = token_text("id_token")
        .map(|token| id_claims(&token))
        .unwrap_or_default();
    let stored_account = token_text("account_id");
    let account_id = claims.account_id.clone().or_else(|| stored_account.clone());
    let identity = AccountIdentity {
        email: claims.email,
        plan: claims.plan.as_deref().map(plan_label),
        stable_key: stable_key(claims.user_id.as_deref(), account_id.as_deref())?,
    };
    Ok(Credentials {
        expires_at: expires_at(&access_token),
        access_token,
        account_id: stored_account.or(account_id),
        identity,
    })
}

fn missing_token_error(document: &Value) -> ProviderError {
    if non_empty(document, "OPENAI_API_KEY").is_some() {
        ProviderError::ApiKeyOnly
    } else {
        ProviderError::NotSignedIn
    }
}

fn stable_key(user_id: Option<&str>, account_id: Option<&str>) -> Result<String, ProviderError> {
    if user_id.is_none() && account_id.is_none() {
        return Err(ProviderError::LocalData(
            "auth.json has no account identity".into(),
        ));
    }
    Ok(format!(
        "{}/{}",
        user_id.unwrap_or_default(),
        account_id.unwrap_or_default()
    ))
}

fn non_empty(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)?
        .as_str()
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(str::to_owned)
}

#[cfg(test)]
#[path = "auth_tests.rs"]
mod tests;
