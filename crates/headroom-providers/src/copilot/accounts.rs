use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use headroom_core::account::{AccountId, AccountRef, CredentialOwner};
use headroom_core::provider::ProviderError;

use super::config::CopilotConfig;
use super::hosts::{GITHUB_HOST, load_users};

pub(super) fn stable_key(login: &str) -> String {
    format!("{GITHUB_HOST}/{}", login.to_ascii_lowercase())
}

fn account_id(login: &str) -> AccountId {
    AccountId::from_stable_key(&super::ID, &stable_key(login))
}

pub(super) fn discover_accounts(config: &CopilotConfig) -> Vec<AccountRef> {
    let mut seen = BTreeSet::new();
    let cli = std::iter::once((config.gh_config_dir.clone(), CredentialOwner::Cli));
    let owned = child_dirs(&config.headroom_accounts_dir())
        .into_iter()
        .map(|dir| (dir, CredentialOwner::Headroom));
    cli.chain(owned)
        .flat_map(|(dir, owner)| accounts_in(&dir, owner))
        .filter(|account| seen.insert(account.id.clone()))
        .collect()
}

pub(super) fn headroom_account_at(home: &Path) -> Result<Option<AccountRef>, ProviderError> {
    let users = load_users(home)?;
    Ok(users
        .0
        .first()
        .map(|login| account_ref(login, home, CredentialOwner::Headroom)))
}

/// The GitHub login behind an account, as listed in its config dir right now.
pub(super) fn login_for(account: &AccountRef) -> Result<String, ProviderError> {
    load_users(&account.home)?
        .0
        .into_iter()
        .find(|login| account_id(login) == account.id)
        .ok_or(ProviderError::NotSignedIn)
}

fn accounts_in(dir: &Path, owner: CredentialOwner) -> Vec<AccountRef> {
    match load_users(dir) {
        Ok(users) => users
            .0
            .iter()
            .map(|login| account_ref(login, dir, owner))
            .collect(),
        Err(error) => {
            tracing::warn!(dir = %dir.display(), %error, "skipping GitHub CLI config");
            Vec::new()
        }
    }
}

fn account_ref(login: &str, dir: &Path, owner: CredentialOwner) -> AccountRef {
    AccountRef {
        id: account_id(login),
        provider: super::ID,
        home: dir.to_path_buf(),
        owner,
    }
}

fn child_dirs(parent: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(parent) else {
        return Vec::new();
    };
    let mut dirs: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect();
    dirs.sort();
    dirs
}

#[cfg(test)]
#[path = "accounts_tests.rs"]
mod tests;
