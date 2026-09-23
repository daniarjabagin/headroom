use std::fs;
use std::io;
use std::path::Path;

use headroom_core::account::AccountIdentity;
use headroom_core::provider::ProviderError;
use headroom_core::secret::SecretString;
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};

pub(super) const AUTH_FILE: &str = "auth.json";
const GO_ENTRY: &str = "opencode-go";
const PLAN: &str = "Go";
const STABLE_KEY_PREFIX: &str = "key-sha256:";

pub(super) fn identity_for_key(key: &str) -> AccountIdentity {
    AccountIdentity {
        email: None,
        plan: Some(PLAN.to_owned()),
        stable_key: format!(
            "{STABLE_KEY_PREFIX}{}",
            hex::encode(Sha256::digest(key.as_bytes()))
        ),
    }
}

pub(super) fn read_cli_key(data_dir: &Path) -> Result<Option<SecretString>, ProviderError> {
    let path = data_dir.join(AUTH_FILE);
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
    Ok(go_key(&entries))
}

fn go_key(entries: &Map<String, Value>) -> Option<SecretString> {
    let key = entries.get(GO_ENTRY)?.get("key")?.as_str()?.trim();
    (!key.is_empty()).then(|| SecretString::new(key.to_owned()))
}

#[cfg(test)]
mod tests {
    use super::*;

    const AUTH: &str = include_str!("fixtures/auth.json");

    fn data_dir_with(text: &str) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join(AUTH_FILE), text).unwrap();
        dir
    }

    #[test]
    fn the_go_entry_key_is_read() {
        let dir = data_dir_with(AUTH);
        let key = read_cli_key(dir.path()).unwrap().unwrap();
        assert_eq!(key.expose(), "sk-fake-go-key");
    }

    #[test]
    fn a_missing_file_or_entry_means_no_key() {
        let empty = tempfile::tempdir().unwrap();
        assert_eq!(read_cli_key(empty.path()), Ok(None));
        let zen_only = data_dir_with(r#"{"opencode":{"type":"api","key":"sk-zen"}}"#);
        assert_eq!(read_cli_key(zen_only.path()), Ok(None));
        let blank = data_dir_with(r#"{"opencode-go":{"type":"api","key":"  "},"v":2}"#);
        assert_eq!(read_cli_key(blank.path()), Ok(None));
        let odd = data_dir_with(r#"{"opencode-go":"sk-x"}"#);
        assert_eq!(read_cli_key(odd.path()), Ok(None));
    }

    #[test]
    fn a_broken_file_is_an_error_not_a_logout() {
        let dir = data_dir_with("{");
        let error = read_cli_key(dir.path()).unwrap_err();
        assert!(matches!(error, ProviderError::LocalData(ref m) if m.contains("auth.json")));
        assert!(!format!("{error:?}").contains("sk-"));
    }

    #[test]
    fn identities_hash_the_key_and_never_contain_it() {
        let identity = identity_for_key("sk-fake-go-key");
        assert_eq!(identity, identity_for_key("sk-fake-go-key"));
        assert_ne!(identity.stable_key, identity_for_key("sk-other").stable_key);
        assert!(identity.stable_key.starts_with(STABLE_KEY_PREFIX));
        assert!(!identity.stable_key.contains("sk-fake"));
        assert_eq!(identity.email, None);
        assert_eq!(identity.plan.as_deref(), Some("Go"));
    }
}
