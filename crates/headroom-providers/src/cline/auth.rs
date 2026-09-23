use std::fmt;
use std::fs;
use std::io;
use std::path::Path;

use headroom_core::account::AccountId;
use headroom_core::provider::ProviderError;
use jiff::{SignedDuration, Timestamp};
use serde::Deserialize;

pub(super) const WORKOS_PREFIX: &str = "workos:";
const EXPIRY_MARGIN: SignedDuration = SignedDuration::from_secs(60);

pub(super) struct Secret(String);

impl Secret {
    pub(super) fn new(value: String) -> Secret {
        Secret(value)
    }

    pub(super) fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Secret(<redacted>)")
    }
}

#[derive(Debug)]
pub(super) struct Credentials {
    pub(super) access_token: Secret,
    pub(super) refresh_token: Option<Secret>,
    pub(super) expires_at: Option<Timestamp>,
    pub(super) user_id: String,
    pub(super) email: Option<String>,
}

impl Credentials {
    pub(super) fn account_id(&self) -> AccountId {
        AccountId::from_stable_key(&super::ID, &self.user_id)
    }

    pub(super) fn is_fresh(&self, now: Timestamp) -> bool {
        self.expires_at.is_some_and(|at| at > now + EXPIRY_MARGIN)
    }
}

#[derive(Deserialize)]
struct RawFile {
    providers: Option<RawProviders>,
}

#[derive(Deserialize)]
struct RawProviders {
    cline: Option<RawEntry>,
}

#[derive(Deserialize)]
struct RawEntry {
    settings: Option<RawSettings>,
}

#[derive(Deserialize)]
struct RawSettings {
    auth: Option<RawAuth>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawAuth {
    access_token: Option<String>,
    refresh_token: Option<String>,
    expires_at: Option<i64>,
    account_id: Option<String>,
    metadata: Option<RawMetadata>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawMetadata {
    user_info: Option<RawUserInfo>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawUserInfo {
    cline_user_id: Option<String>,
    email: Option<String>,
}

pub(super) fn read_file(path: &Path) -> Result<Option<String>, ProviderError> {
    match fs::read_to_string(path) {
        Ok(text) => Ok(Some(text)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(ProviderError::LocalData(format!(
            "cannot read {}: {}",
            path.display(),
            error.kind()
        ))),
    }
}

pub(super) fn parse_credentials(text: &str) -> Result<Credentials, ProviderError> {
    let file: RawFile = serde_json::from_str(text).map_err(|error| {
        ProviderError::LocalData(format!(
            "cannot parse Cline providers.json at line {} column {}",
            error.line(),
            error.column()
        ))
    })?;
    let auth = file
        .providers
        .and_then(|providers| providers.cline)
        .and_then(|entry| entry.settings)
        .and_then(|settings| settings.auth)
        .ok_or(ProviderError::NotSignedIn)?;
    credentials_of(auth)
}

fn credentials_of(auth: RawAuth) -> Result<Credentials, ProviderError> {
    let access = non_empty(auth.access_token).ok_or(ProviderError::NotSignedIn)?;
    let user_info = auth.metadata.and_then(|metadata| metadata.user_info);
    let (info_id, email) = match user_info {
        Some(info) => (non_empty(info.cline_user_id), non_empty(info.email)),
        None => (None, None),
    };
    let user_id = non_empty(auth.account_id)
        .or(info_id)
        .ok_or_else(|| ProviderError::LocalData("Cline sign-in has no account id".to_owned()))?;
    Ok(Credentials {
        access_token: Secret(bearer_value(&access)),
        refresh_token: non_empty(auth.refresh_token).map(Secret),
        expires_at: auth
            .expires_at
            .and_then(|millis| Timestamp::from_millisecond(millis).ok()),
        user_id,
        email,
    })
}

pub(super) fn bearer_value(token: &str) -> String {
    let token = token.trim();
    let prefixed = token
        .get(..WORKOS_PREFIX.len())
        .is_some_and(|head| head.eq_ignore_ascii_case(WORKOS_PREFIX));
    if prefixed {
        token.to_owned()
    } else {
        format!("{WORKOS_PREFIX}{token}")
    }
}

fn non_empty(value: Option<String>) -> Option<String> {
    value
        .map(|text| text.trim().to_owned())
        .filter(|text| !text.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    const SIGNED_IN: &str = include_str!("fixtures/providers.json");

    fn now() -> Timestamp {
        "2026-09-23T10:00:00Z".parse().unwrap()
    }

    #[test]
    fn the_cline_oauth_entry_is_read() {
        let credentials = parse_credentials(SIGNED_IN).unwrap();
        assert_eq!(credentials.access_token.expose(), "workos:fake.jwt.token");
        assert_eq!(credentials.refresh_token.unwrap().expose(), "fake-refresh");
        assert_eq!(credentials.user_id, "usr-0000000000000001");
        assert_eq!(credentials.email.as_deref(), Some("someone@example.com"));
        assert_eq!(
            credentials.expires_at,
            Some("2026-09-23T10:30:00Z".parse().unwrap())
        );
        assert!(format!("{:?}", credentials.access_token).contains("redacted"));
    }

    #[test]
    fn freshness_keeps_a_safety_margin() {
        let mut credentials = parse_credentials(SIGNED_IN).unwrap();
        assert!(credentials.is_fresh(now()));
        credentials.expires_at = Some(now() + SignedDuration::from_secs(30));
        assert!(!credentials.is_fresh(now()));
        credentials.expires_at = None;
        assert!(!credentials.is_fresh(now()));
    }

    #[test]
    fn tokens_are_sent_with_the_workos_prefix_once() {
        assert_eq!(bearer_value("abc"), "workos:abc");
        assert_eq!(bearer_value(" workos:abc "), "workos:abc");
        assert_eq!(bearer_value("WorkOS:abc"), "WorkOS:abc");
    }

    #[test]
    fn other_providers_or_no_token_mean_signed_out() {
        let byo = r#"{"version":1,"providers":{"anthropic":{"settings":{"apiKey":"k"}}}}"#;
        assert_eq!(
            parse_credentials(byo).unwrap_err(),
            ProviderError::NotSignedIn
        );
        let empty = r#"{"providers":{"cline":{"settings":{"auth":{"accessToken":" "}}}}}"#;
        assert_eq!(
            parse_credentials(empty).unwrap_err(),
            ProviderError::NotSignedIn
        );
    }

    #[test]
    fn the_account_id_falls_back_to_user_info() {
        let text = r#"{"providers":{"cline":{"settings":{"auth":{"accessToken":"t",
            "metadata":{"userInfo":{"clineUserId":"usr-2"}}}}}}}"#;
        assert_eq!(parse_credentials(text).unwrap().user_id, "usr-2");
        let missing = r#"{"providers":{"cline":{"settings":{"auth":{"accessToken":"t"}}}}}"#;
        assert!(matches!(
            parse_credentials(missing),
            Err(ProviderError::LocalData(_))
        ));
        assert!(matches!(
            parse_credentials("{"),
            Err(ProviderError::LocalData(_))
        ));
    }
}
