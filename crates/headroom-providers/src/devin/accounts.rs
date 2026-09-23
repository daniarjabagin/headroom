use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use headroom_core::account::{AccountRef, CredentialOwner};
use headroom_core::provider::ProviderError;

use super::auth::{DevinKey, load_key};
use super::config::DevinConfig;

struct Candidate {
    home: PathBuf,
    owner: CredentialOwner,
}

pub(super) fn discover_accounts(config: &DevinConfig) -> Vec<AccountRef> {
    let mut seen = BTreeSet::new();
    candidates(config)
        .into_iter()
        .filter_map(inspect)
        .filter(|account| seen.insert(account.id.clone()))
        .collect()
}

pub(super) fn headroom_account_at(home: &Path) -> Result<Option<AccountRef>, ProviderError> {
    let candidate = Candidate {
        home: home.to_path_buf(),
        owner: CredentialOwner::Headroom,
    };
    let key = load_key(&key_dir(&candidate.home, candidate.owner))?;
    Ok(key.map(|key| account_ref(&key, candidate)))
}

pub(super) fn key_dir(home: &Path, owner: CredentialOwner) -> PathBuf {
    match owner {
        CredentialOwner::Cli => home.to_path_buf(),
        CredentialOwner::Headroom => home.join(super::DATA_SUBDIR),
    }
}

fn candidates(config: &DevinConfig) -> Vec<Candidate> {
    let cli = [config.cli_dir(), config.app_state_dir()]
        .into_iter()
        .map(|home| Candidate {
            home,
            owner: CredentialOwner::Cli,
        });
    let owned = child_dirs(&config.headroom_accounts_dir())
        .into_iter()
        .map(|home| Candidate {
            home,
            owner: CredentialOwner::Headroom,
        });
    cli.chain(owned).collect()
}

fn inspect(candidate: Candidate) -> Option<AccountRef> {
    match load_key(&key_dir(&candidate.home, candidate.owner)) {
        Ok(key) => key.map(|key| account_ref(&key, candidate)),
        Err(error) => {
            tracing::warn!(dir = %candidate.home.display(), %error, "skipping Devin sign-in");
            None
        }
    }
}

fn account_ref(key: &DevinKey, candidate: Candidate) -> AccountRef {
    AccountRef {
        id: key.account_id(),
        provider: super::ID,
        home: candidate.home,
        owner: candidate.owner,
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
