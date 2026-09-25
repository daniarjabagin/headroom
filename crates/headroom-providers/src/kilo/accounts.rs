use std::fs;
use std::path::{Path, PathBuf};

use headroom_core::account::{AccountRef, CredentialOwner};
use headroom_core::provider::ProviderError;

use super::auth::{KiloToken, read_login};
use super::config::KiloConfig;
use crate::key_accounts::{self, RECORD_FILE};

pub(super) enum Credential {
    Key,
    Login(KiloToken),
}

pub(super) fn discover(config: &KiloConfig) -> Result<Vec<AccountRef>, ProviderError> {
    let mut accounts = key_accounts::discover(&config.accounts_dir, &super::ID)?;
    accounts.extend(inspect(&config.data_dir, CredentialOwner::Cli));
    accounts.extend(
        login_homes(&config.accounts_dir)
            .iter()
            .filter_map(|home| inspect(home, CredentialOwner::Headroom)),
    );
    Ok(accounts)
}

pub(super) fn headroom_account_at(home: &Path) -> Result<Option<AccountRef>, ProviderError> {
    let login = read_login(&login_dir(home, CredentialOwner::Headroom))?;
    Ok(login.map(|login| account_ref(&login, home, CredentialOwner::Headroom)))
}

pub(super) fn credential(account: &AccountRef) -> Result<Credential, ProviderError> {
    if account.owner == CredentialOwner::Headroom && account.home.join(RECORD_FILE).is_file() {
        return Ok(Credential::Key);
    }
    let login =
        read_login(&login_dir(&account.home, account.owner))?.ok_or(ProviderError::NotSignedIn)?;
    if login.account_id() == account.id {
        Ok(Credential::Login(login))
    } else {
        Err(ProviderError::AccountChanged(format!(
            "the Kilo sign-in at {} has changed",
            account.home.display()
        )))
    }
}

fn login_dir(home: &Path, owner: CredentialOwner) -> PathBuf {
    match owner {
        CredentialOwner::Cli => home.to_path_buf(),
        CredentialOwner::Headroom => home.join(super::DATA_SUBDIR),
    }
}

fn inspect(home: &Path, owner: CredentialOwner) -> Option<AccountRef> {
    match read_login(&login_dir(home, owner)) {
        Ok(login) => login.map(|login| account_ref(&login, home, owner)),
        Err(error) => {
            tracing::warn!(dir = %home.display(), %error, "skipping Kilo sign-in");
            None
        }
    }
}

fn account_ref(login: &KiloToken, home: &Path, owner: CredentialOwner) -> AccountRef {
    AccountRef {
        id: login.account_id(),
        provider: super::ID,
        home: home.to_path_buf(),
        owner,
    }
}

fn login_homes(root: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(root) else {
        return Vec::new();
    };
    let mut homes: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|home| home.is_dir() && !home.join(RECORD_FILE).exists())
        .collect();
    homes.sort();
    homes
}

#[cfg(test)]
#[path = "accounts_tests.rs"]
mod tests;
