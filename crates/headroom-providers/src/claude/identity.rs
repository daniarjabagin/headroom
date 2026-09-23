use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use headroom_core::account::{AccountId, ProviderKind};
use headroom_core::provider::ProviderError;
use serde::Deserialize;

const IDENTITY_FILE: &str = ".claude.json";
const DEFAULT_DIR_NAME: &str = ".claude";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ClaudeIdentity {
    pub(super) stable_key: String,
    pub(super) email: Option<String>,
}

impl ClaudeIdentity {
    pub(super) fn account_id(&self) -> AccountId {
        AccountId::from_stable_key(ProviderKind::Claude, &self.stable_key)
    }
}

#[derive(Deserialize)]
struct RawState {
    #[serde(rename = "oauthAccount")]
    oauth_account: Option<RawOAuthAccount>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawOAuthAccount {
    account_uuid: Option<String>,
    organization_uuid: Option<String>,
    email_address: Option<String>,
}

pub(super) fn identity_file(home: &Path, dir: &Path) -> PathBuf {
    if same_dir(dir, &home.join(DEFAULT_DIR_NAME)) {
        home.join(IDENTITY_FILE)
    } else {
        dir.join(IDENTITY_FILE)
    }
}

pub(super) fn same_dir(a: &Path, b: &Path) -> bool {
    canonical(a) == canonical(b)
}

pub(super) fn canonical(path: &Path) -> PathBuf {
    fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

pub(super) fn load_identity(
    home: &Path,
    dir: &Path,
) -> Result<Option<ClaudeIdentity>, ProviderError> {
    let path = identity_file(home, dir);
    match fs::read_to_string(&path) {
        Ok(text) => parse_identity(&text).map_err(|message| {
            ProviderError::LocalData(format!("cannot parse {}: {message}", path.display()))
        }),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(ProviderError::LocalData(format!(
            "cannot read {}: {}",
            path.display(),
            error.kind()
        ))),
    }
}

pub(super) fn parse_identity(text: &str) -> Result<Option<ClaudeIdentity>, String> {
    let state: RawState = serde_json::from_str(text)
        .map_err(|error| format!("line {} column {}", error.line(), error.column()))?;
    Ok(state.oauth_account.and_then(identity_of))
}

fn identity_of(account: RawOAuthAccount) -> Option<ClaudeIdentity> {
    let account_uuid = non_empty(account.account_uuid)?;
    let organization_uuid = non_empty(account.organization_uuid).unwrap_or_default();
    Some(ClaudeIdentity {
        stable_key: format!(
            "{}/{}",
            account_uuid.to_lowercase(),
            organization_uuid.to_lowercase()
        ),
        email: non_empty(account.email_address),
    })
}

fn non_empty(value: Option<String>) -> Option<String> {
    value
        .map(|text| text.trim().to_owned())
        .filter(|text| !text.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    const STATE: &str = r#"{
        "numStartups": 3,
        "oauthAccount": {
            "accountUuid": "AAAAAAAA-0000-4000-8000-000000000001",
            "organizationUuid": "bbbbbbbb-0000-4000-8000-000000000002",
            "emailAddress": "someone@example.com",
            "organizationName": "Example Org"
        }
    }"#;

    #[test]
    fn identity_uses_account_and_organization() {
        let identity = parse_identity(STATE).unwrap().unwrap();
        assert_eq!(
            identity.stable_key,
            "aaaaaaaa-0000-4000-8000-000000000001/bbbbbbbb-0000-4000-8000-000000000002"
        );
        assert_eq!(identity.email.as_deref(), Some("someone@example.com"));
        assert!(identity.account_id().0.starts_with("claude:"));
    }

    #[test]
    fn missing_oauth_account_is_no_identity() {
        assert_eq!(parse_identity(r#"{"numStartups": 1}"#).unwrap(), None);
        assert_eq!(
            parse_identity(r#"{"oauthAccount": {"accountUuid": ""}}"#).unwrap(),
            None
        );
    }

    #[test]
    fn garbage_is_an_error_without_contents() {
        let message = parse_identity("{\"oauthAccount\": 7, \"secret\": 1").unwrap_err();
        assert!(message.starts_with("line 1"));
    }

    #[test]
    fn default_dir_reads_identity_next_to_it() {
        let home = tempfile::tempdir().unwrap();
        let default_dir = home.path().join(".claude");
        let other = home.path().join(".claude-work");
        fs::create_dir_all(&default_dir).unwrap();
        assert_eq!(
            identity_file(home.path(), &default_dir),
            home.path().join(".claude.json")
        );
        assert_eq!(
            identity_file(home.path(), &other),
            other.join(".claude.json")
        );
    }

    #[test]
    fn missing_identity_file_is_none() {
        let home = tempfile::tempdir().unwrap();
        let dir = home.path().join(".claude");
        assert_eq!(load_identity(home.path(), &dir).unwrap(), None);
    }
}
