use std::fs;
use std::io;
use std::path::Path;

use headroom_core::provider::ProviderError;
use sha2::{Digest, Sha256};

use super::auth::{AUTH_FILE, Credentials, credentials_from_keychain, load_credentials};
use crate::homes::canonical;
use crate::keychain::{GenericPassword, Security};
use crate::toml::top_level_string;

const CONFIG_FILE: &str = "config.toml";
const STORE_KEY: &str = "cli_auth_credentials_store";
const SERVICE: &str = "Codex Auth";
const STORE_KEY_HEX_LEN: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum StoreMode {
    File,
    Keyring,
    Auto,
}

pub(super) async fn load(
    keychain: Option<&Security>,
    home: &Path,
) -> Result<Credentials, ProviderError> {
    match keychain {
        Some(security) if !reads_file(keychain, home)? => load_from_keychain(security, home).await,
        _ => load_credentials(home),
    }
}

pub(super) fn reads_file(keychain: Option<&Security>, home: &Path) -> Result<bool, ProviderError> {
    if keychain.is_none() {
        return Ok(true);
    }
    Ok(match store_mode(home)? {
        StoreMode::File => true,
        StoreMode::Keyring => false,
        StoreMode::Auto => home.join(AUTH_FILE).exists(),
    })
}

pub(super) fn store_mode(home: &Path) -> Result<StoreMode, ProviderError> {
    let path = home.join(CONFIG_FILE);
    match fs::read_to_string(&path) {
        Ok(text) => Ok(parse_store_mode(&text)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(StoreMode::File),
        Err(error) => Err(ProviderError::LocalData(format!(
            "cannot read {}: {}",
            path.display(),
            error.kind()
        ))),
    }
}

fn parse_store_mode(text: &str) -> StoreMode {
    match top_level_string(text, STORE_KEY).as_deref() {
        Some("keyring") => StoreMode::Keyring,
        Some("auto") => StoreMode::Auto,
        _ => StoreMode::File,
    }
}

pub(super) fn store_key(home: &Path) -> String {
    let path = canonical(home);
    let digest = hex::encode(Sha256::digest(path.to_string_lossy().as_bytes()));
    let truncated = digest.get(..STORE_KEY_HEX_LEN).unwrap_or(&digest);
    format!("cli|{truncated}")
}

async fn load_from_keychain(
    security: &Security,
    home: &Path,
) -> Result<Credentials, ProviderError> {
    let account = store_key(home);
    let item = GenericPassword {
        service: SERVICE,
        account: Some(&account),
    };
    let found = security.find(item).await.map_err(|error| {
        ProviderError::LocalData(format!(
            "cannot read the Codex sign-in from the Keychain: {error}"
        ))
    })?;
    match found {
        Some(secret) => credentials_from_keychain(secret.expose().trim().as_bytes()),
        None => load_credentials(home),
    }
}

#[cfg(test)]
#[path = "keyring_tests.rs"]
mod tests;
