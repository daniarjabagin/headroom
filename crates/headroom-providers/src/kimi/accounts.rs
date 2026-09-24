use std::fs;
use std::path::{Path, PathBuf};

use headroom_core::account::{AccountIdentity, AccountRef, CredentialOwner};
use headroom_core::provider::ProviderError;

use super::ID;
use super::config::KimiConfig;
use super::credentials::has_credentials;
use crate::homes::canonical;
use crate::key_accounts::{self, RECORD_FILE};

pub(super) enum Credential {
    Key(AccountIdentity),
    OAuth(AccountIdentity),
}

pub(super) fn discover(config: &KimiConfig) -> Result<Vec<AccountRef>, ProviderError> {
    let mut accounts = key_accounts::discover(&config.accounts_dir, &ID)?;
    if has_credentials(&config.share_dir) {
        accounts.push(oauth_account(&config.share_dir, CredentialOwner::Cli));
    }
    accounts.extend(
        oauth_homes(&config.accounts_dir)
            .iter()
            .map(|home| oauth_account(home, CredentialOwner::Headroom)),
    );
    Ok(accounts)
}

pub(super) fn credential(home: &Path) -> Result<Credential, ProviderError> {
    if let Some(identity) = key_accounts::load_record(home)? {
        return Ok(Credential::Key(identity));
    }
    if has_credentials(home) {
        return Ok(Credential::OAuth(oauth_identity(home)));
    }
    Err(ProviderError::NotSignedIn)
}

pub(super) fn key_identity(key: &str, plan: Option<String>) -> AccountIdentity {
    AccountIdentity {
        email: None,
        plan,
        stable_key: key_accounts::fingerprint_stable_key(key),
    }
}

fn oauth_identity(home: &Path) -> AccountIdentity {
    AccountIdentity {
        email: None,
        plan: None,
        stable_key: format!("oauth:{}", canonical(home).display()),
    }
}

fn oauth_account(home: &Path, owner: CredentialOwner) -> AccountRef {
    AccountRef {
        id: oauth_identity(home).account_id(&ID),
        provider: ID,
        home: home.to_path_buf(),
        owner,
    }
}

fn oauth_homes(root: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(root) else {
        return Vec::new();
    };
    let mut homes: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|home| home.is_dir() && has_credentials(home))
        .filter(|home| !home.join(RECORD_FILE).exists())
        .collect();
    homes.sort();
    homes
}

#[cfg(test)]
#[path = "accounts_tests.rs"]
mod tests;
