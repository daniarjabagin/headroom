use std::path::Path;

use headroom_core::account::AccountRef;
use headroom_core::provider::ProviderError;
use headroom_core::secret::SecretString;

use super::auth::{CREDENTIALS_FILE, Credentials, load_credentials, parse_credentials};
use super::config::ClaudeConfig;
use super::identity::identity_file;
use super::keychain::{self, Candidate};
use crate::keychain::{GenericPassword, Security};

pub(super) async fn load(
    config: &ClaudeConfig,
    account: &AccountRef,
) -> Result<Credentials, ProviderError> {
    let Some(security) = &config.keychain else {
        return load_credentials(&account.home);
    };
    let candidates = keychain::candidates(config, &account.home, account.owner);
    match find_first(security, &candidates).await? {
        Some(text) => parse_keychain_item(text.expose()),
        None => load_credentials(&account.home),
    }
}

pub(super) fn has_sign_in(config: &ClaudeConfig, dir: &Path) -> bool {
    dir.join(CREDENTIALS_FILE).is_file()
        || (config.keychain.is_some() && identity_file(config, dir).is_file())
}

async fn find_first(
    security: &Security,
    candidates: &[Candidate],
) -> Result<Option<SecretString>, ProviderError> {
    for candidate in candidates {
        let item = GenericPassword {
            service: &candidate.service,
            account: candidate.account.as_deref(),
        };
        let found = security.find(item).await.map_err(|error| {
            ProviderError::LocalData(format!(
                "cannot read the Claude sign-in from the Keychain: {error}"
            ))
        })?;
        if found.is_some() {
            return Ok(found);
        }
    }
    Ok(None)
}

fn parse_keychain_item(text: &str) -> Result<Credentials, ProviderError> {
    let text = text.trim();
    if text.starts_with('{') {
        return parse_credentials(text);
    }
    let decoded = hex::decode(text)
        .ok()
        .and_then(|bytes| String::from_utf8(bytes).ok())
        .ok_or_else(|| {
            ProviderError::LocalData("the Claude Keychain item is not valid JSON".to_owned())
        })?;
    parse_credentials(&decoded)
}

#[cfg(test)]
#[path = "credentials_tests.rs"]
mod tests;
