use std::fs;
use std::io;
use std::path::Path;

use headroom_core::account::AccountId;
use headroom_core::provider::ProviderError;
use headroom_core::secret::SecretString;
use sha2::{Digest, Sha256};

use super::state_db::{STATE_DB_FILE, read_app_key};
use crate::toml::top_level_string;

pub(super) const CREDENTIALS_FILE: &str = "credentials.toml";
const KEY_FIELD: &str = "windsurf_api_key";
const SERVER_FIELD: &str = "api_server_url";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct DevinKey {
    pub(super) api_key: SecretString,
    pub(super) api_server: Option<String>,
}

impl DevinKey {
    pub(super) fn stable_key(&self) -> String {
        let digest = Sha256::digest(self.api_key.expose().as_bytes());
        format!("devin-key-sha256:{}", hex::encode(digest))
    }

    pub(super) fn account_id(&self) -> AccountId {
        AccountId::from_stable_key(&super::ID, &self.stable_key())
    }
}

pub(super) fn load_key(dir: &Path) -> Result<Option<DevinKey>, ProviderError> {
    let credentials = dir.join(CREDENTIALS_FILE);
    if let Some(text) = read_optional(&credentials)? {
        return parse_credentials(&text, &credentials);
    }
    let state_db = dir.join(STATE_DB_FILE);
    if !state_db.is_file() {
        return Ok(None);
    }
    Ok(read_app_key(&state_db)?.map(|key| DevinKey {
        api_key: SecretString::new(key),
        api_server: None,
    }))
}

fn read_optional(path: &Path) -> Result<Option<String>, ProviderError> {
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

pub(super) fn parse_credentials(
    text: &str,
    path: &Path,
) -> Result<Option<DevinKey>, ProviderError> {
    let Some(key) = top_level_string(text, KEY_FIELD)
        .map(|key| key.trim().to_owned())
        .filter(|key| !key.is_empty())
    else {
        return Ok(None);
    };
    let api_server = top_level_string(text, SERVER_FIELD)
        .map(|url| server_url(&url, path))
        .transpose()?
        .flatten();
    Ok(Some(DevinKey {
        api_key: SecretString::new(key),
        api_server,
    }))
}

fn server_url(raw: &str, path: &Path) -> Result<Option<String>, ProviderError> {
    let trimmed = raw.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        return Ok(None);
    }
    if trimmed.starts_with("https://") && trimmed.len() > "https://".len() {
        return Ok(Some(trimmed.to_owned()));
    }
    Err(ProviderError::LocalData(format!(
        "{SERVER_FIELD} in {} must be an https URL",
        path.display()
    )))
}

#[cfg(test)]
mod tests {
    use super::super::state_db::tests::write_state_db;
    use super::*;

    const CREDENTIALS: &str = include_str!("fixtures/credentials.toml");

    fn parse(text: &str) -> Result<Option<DevinKey>, ProviderError> {
        parse_credentials(text, Path::new("/x/credentials.toml"))
    }

    #[test]
    fn the_cli_credentials_give_the_key_and_server() {
        let key = parse(CREDENTIALS).unwrap().unwrap();
        assert_eq!(
            key.api_key.expose(),
            "devin-session-token$eyJhbGciOiJIUzI1NiJ9.e30.fake-signature"
        );
        assert_eq!(key.api_server, None);
        let enterprise =
            parse("windsurf_api_key = \"k\"\napi_server_url = \"https://devin.corp.example/ \"\n")
                .unwrap()
                .unwrap();
        assert_eq!(
            enterprise.api_server.as_deref(),
            Some("https://devin.corp.example")
        );
    }

    #[test]
    fn a_missing_or_blank_key_is_signed_out() {
        assert_eq!(parse("api_server_url = \"https://x.example\""), Ok(None));
        assert_eq!(parse("windsurf_api_key = \"  \""), Ok(None));
    }

    #[test]
    fn a_non_https_server_is_refused_instead_of_sending_the_key_elsewhere() {
        let error =
            parse("windsurf_api_key = \"k\"\napi_server_url = \"http://x.example\"").unwrap_err();
        assert_eq!(
            error,
            ProviderError::LocalData(
                "api_server_url in /x/credentials.toml must be an https URL".into()
            )
        );
    }

    #[test]
    fn the_account_id_hashes_the_key_and_never_contains_it() {
        let key = parse("windsurf_api_key = \"secret-1\"").unwrap().unwrap();
        assert!(key.stable_key().starts_with("devin-key-sha256:"));
        assert!(!key.stable_key().contains("secret-1"));
        assert!(key.account_id().0.starts_with("devin:"));
        let other = parse("windsurf_api_key = \"secret-2\"").unwrap().unwrap();
        assert_ne!(key.account_id(), other.account_id());
    }

    #[test]
    fn the_cli_file_wins_over_the_app_database_in_the_same_dir() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(load_key(dir.path()), Ok(None));
        write_state_db(&dir.path().join(STATE_DB_FILE), Some(r#"{"apiKey":"app"}"#));
        assert_eq!(
            load_key(dir.path()).unwrap().unwrap().api_key.expose(),
            "app"
        );
        fs::write(
            dir.path().join(CREDENTIALS_FILE),
            "windsurf_api_key = 'cli'",
        )
        .unwrap();
        assert_eq!(
            load_key(dir.path()).unwrap().unwrap().api_key.expose(),
            "cli"
        );
    }
}
