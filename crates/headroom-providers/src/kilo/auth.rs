use std::fs;
use std::io;
use std::path::Path;

use headroom_core::account::{AccountId, AccountIdentity};
use headroom_core::provider::ProviderError;
use headroom_core::secret::SecretString;
use serde_json::{Map, Value};

use crate::key_accounts;

pub(super) const AUTH_FILE: &str = "auth.json";
const KILO_ENTRY: &str = "kilo";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct KiloToken {
    pub(super) token: SecretString,
    pub(super) organization_id: Option<String>,
}

impl KiloToken {
    pub(super) fn stable_key(&self) -> String {
        let key = key_accounts::sha256_stable_key(self.token.expose());
        match &self.organization_id {
            Some(organization) => format!("{key}/org:{organization}"),
            None => key,
        }
    }

    pub(super) fn identity(&self) -> AccountIdentity {
        AccountIdentity {
            email: None,
            plan: None,
            stable_key: self.stable_key(),
        }
    }

    pub(super) fn account_id(&self) -> AccountId {
        self.identity().account_id(&super::ID)
    }
}

pub(super) fn read_login(dir: &Path) -> Result<Option<KiloToken>, ProviderError> {
    let path = dir.join(AUTH_FILE);
    let text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(ProviderError::LocalData(format!(
                "cannot read {}: {}",
                path.display(),
                error.kind()
            )));
        }
    };
    let entries: Map<String, Value> = serde_json::from_str(&text).map_err(|error| {
        ProviderError::LocalData(format!(
            "cannot parse {} at line {} column {}",
            path.display(),
            error.line(),
            error.column()
        ))
    })?;
    Ok(entries.get(KILO_ENTRY).and_then(kilo_token))
}

fn kilo_token(entry: &Value) -> Option<KiloToken> {
    let field = match entry.get("type")?.as_str()? {
        "oauth" => "access",
        "api" => "key",
        _ => return None,
    };
    let token = non_blank(entry.get(field))?;
    Some(KiloToken {
        token: SecretString::new(token),
        organization_id: non_blank(entry.get("accountId")),
    })
}

fn non_blank(value: Option<&Value>) -> Option<String> {
    let text = value?.as_str()?.trim();
    (!text.is_empty()).then(|| text.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    const AUTH: &str = include_str!("fixtures/auth.json");
    const AUTH_ORG: &str = include_str!("fixtures/auth_organization.json");

    fn dir_with(text: &str) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join(AUTH_FILE), text).unwrap();
        dir
    }

    #[test]
    fn the_device_login_token_is_read() {
        let dir = dir_with(AUTH);
        let login = read_login(dir.path()).unwrap().unwrap();
        assert_eq!(login.token.expose(), "kilo-fake-access-token");
        assert_eq!(login.organization_id, None);
    }

    #[test]
    fn an_organization_login_keeps_its_organization() {
        let dir = dir_with(AUTH_ORG);
        let login = read_login(dir.path()).unwrap().unwrap();
        assert_eq!(login.organization_id.as_deref(), Some("org-0000-fake"));
        let personal = read_login(dir_with(AUTH).path()).unwrap().unwrap();
        assert_ne!(login.account_id(), personal.account_id());
        assert!(login.stable_key().ends_with("/org:org-0000-fake"));
    }

    #[test]
    fn an_api_entry_uses_its_key() {
        let dir = dir_with(r#"{"kilo":{"type":"api","key":" kilo-key "}}"#);
        let login = read_login(dir.path()).unwrap().unwrap();
        assert_eq!(login.token.expose(), "kilo-key");
    }

    #[test]
    fn missing_files_entries_or_tokens_mean_signed_out() {
        let empty = tempfile::tempdir().unwrap();
        assert_eq!(read_login(empty.path()), Ok(None));
        for text in [
            r#"{"opencode":{"type":"api","key":"sk"}}"#,
            r#"{"kilo":{"type":"oauth","access":"  "}}"#,
            r#"{"kilo":{"type":"wellknown","key":"k"}}"#,
            r#"{"kilo":"token"}"#,
        ] {
            assert_eq!(read_login(dir_with(text).path()), Ok(None), "{text}");
        }
    }

    #[test]
    fn a_broken_file_is_an_error_that_never_shows_the_token() {
        let dir = dir_with(r#"{"kilo":{"access":"kilo-secret""#);
        let error = read_login(dir.path()).unwrap_err();
        assert!(matches!(error, ProviderError::LocalData(ref m) if m.contains("auth.json")));
        assert!(!format!("{error:?}").contains("kilo-secret"));
    }

    #[test]
    fn identities_hash_the_token() {
        let login = read_login(dir_with(AUTH).path()).unwrap().unwrap();
        let identity = login.identity();
        assert!(identity.stable_key.starts_with("key-sha256:"));
        assert!(!identity.stable_key.contains("kilo-fake"));
        assert_eq!(identity.account_id(&super::super::ID), login.account_id());
    }
}
