use std::fmt;
use std::fs;
use std::io;
use std::path::Path;

use headroom_core::provider::ProviderError;
use jiff::Timestamp;
use serde::Deserialize;

use super::number::whole_number;

pub(super) const CREDENTIALS_FILE: &str = ".credentials.json";
const PROFILE_SCOPE: &str = "user:profile";
const FREE_SUBSCRIPTION: &str = "free";

pub(super) struct AccessToken(String);

impl AccessToken {
    pub(super) fn secret(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for AccessToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("AccessToken(<redacted>)")
    }
}

#[derive(Debug)]
pub(super) struct Credentials {
    access_token: AccessToken,
    expires_at: Option<Timestamp>,
    pub(super) plan: Option<String>,
    pub(super) subscribed: bool,
    has_profile_scope: bool,
}

impl Credentials {
    pub(super) fn token_secret(&self) -> &str {
        self.access_token.secret()
    }

    pub(super) fn is_expired(&self, now: Timestamp) -> bool {
        self.expires_at.is_some_and(|at| at <= now)
    }

    pub(super) fn has_profile_scope(&self) -> bool {
        self.has_profile_scope
    }

    pub(super) fn usable_token(&self, now: Timestamp) -> Result<&AccessToken, ProviderError> {
        if self.is_expired(now) || !self.has_profile_scope {
            return Err(ProviderError::SignInExpired);
        }
        Ok(&self.access_token)
    }
}

#[derive(Deserialize)]
struct RawCredentialsFile {
    #[serde(rename = "claudeAiOauth")]
    oauth: Option<RawOAuth>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawOAuth {
    access_token: Option<String>,
    expires_at: Option<serde_json::Number>,
    subscription_type: Option<String>,
    rate_limit_tier: Option<String>,
    scopes: Option<Vec<String>>,
}

pub(super) fn load_credentials(dir: &Path) -> Result<Credentials, ProviderError> {
    let path = dir.join(CREDENTIALS_FILE);
    match fs::read_to_string(&path) {
        Ok(text) => parse_credentials(&text),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Err(ProviderError::NotSignedIn),
        Err(error) => Err(ProviderError::LocalData(format!(
            "cannot read {}: {}",
            path.display(),
            error.kind()
        ))),
    }
}

pub(super) fn parse_credentials(text: &str) -> Result<Credentials, ProviderError> {
    let file: RawCredentialsFile = serde_json::from_str(text).map_err(|error| {
        ProviderError::LocalData(format!(
            "cannot parse {CREDENTIALS_FILE} at line {} column {}",
            error.line(),
            error.column()
        ))
    })?;
    let oauth = file.oauth.ok_or(ProviderError::NotSignedIn)?;
    let access_token = oauth
        .access_token
        .filter(|token| !token.is_empty())
        .ok_or(ProviderError::NotSignedIn)?;
    Ok(Credentials {
        access_token: AccessToken(access_token),
        expires_at: oauth.expires_at.as_ref().and_then(millis_timestamp),
        plan: oauth
            .subscription_type
            .as_deref()
            .and_then(|kind| plan_label(kind, oauth.rate_limit_tier.as_deref())),
        subscribed: is_subscribed(oauth.subscription_type.as_deref()),
        has_profile_scope: has_profile_scope(oauth.scopes.as_deref()),
    })
}

fn millis_timestamp(millis: &serde_json::Number) -> Option<Timestamp> {
    Timestamp::from_millisecond(whole_number(millis)?).ok()
}

fn is_subscribed(subscription: Option<&str>) -> bool {
    subscription
        .map(str::trim)
        .is_some_and(|kind| !kind.is_empty() && !kind.eq_ignore_ascii_case(FREE_SUBSCRIPTION))
}

fn has_profile_scope(scopes: Option<&[String]>) -> bool {
    scopes.is_none_or(|scopes| scopes.is_empty() || scopes.iter().any(|s| s == PROFILE_SCOPE))
}

pub(super) fn plan_label(subscription: &str, rate_limit_tier: Option<&str>) -> Option<String> {
    let name = title_case_word(subscription.trim())?;
    let multiplier = rate_limit_tier.and_then(tier_multiplier);
    Some(match multiplier {
        Some(multiplier) => format!("{name} {multiplier}"),
        None => name,
    })
}

fn tier_multiplier(tier: &str) -> Option<&str> {
    tier.split(|c: char| !c.is_ascii_alphanumeric())
        .find(|token| is_multiplier(token))
}

fn is_multiplier(token: &str) -> bool {
    token
        .strip_suffix('x')
        .is_some_and(|digits| !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()))
}

pub(super) fn title_case_word(word: &str) -> Option<String> {
    let mut chars = word.chars();
    let first = chars.next()?;
    Some(
        first
            .to_uppercase()
            .chain(chars.flat_map(char::to_lowercase))
            .collect(),
    )
}

#[cfg(test)]
#[path = "auth_tests.rs"]
mod tests;
