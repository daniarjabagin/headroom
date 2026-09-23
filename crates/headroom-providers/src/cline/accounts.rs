use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use headroom_core::account::{AccountRef, CredentialOwner};
use headroom_core::provider::ProviderError;

use super::auth::{parse_credentials, read_file};
use super::config::ClineConfig;

pub(super) fn discover_accounts(config: &ClineConfig) -> Vec<AccountRef> {
    let mut seen = BTreeSet::new();
    candidates(config)
        .into_iter()
        .filter_map(|(home, owner)| inspect(config, home, owner))
        .filter(|account| seen.insert(account.id.clone()))
        .collect()
}

pub(super) fn headroom_account_at(
    config: &ClineConfig,
    home: &Path,
) -> Result<Option<AccountRef>, ProviderError> {
    account_at(config, home.to_path_buf(), CredentialOwner::Headroom)
}

fn candidates(config: &ClineConfig) -> Vec<(PathBuf, CredentialOwner)> {
    let mut list = vec![(config.cli_home(), CredentialOwner::Cli)];
    list.extend(
        child_dirs(&config.headroom_accounts_dir())
            .into_iter()
            .map(|home| (home, CredentialOwner::Headroom)),
    );
    list
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

fn inspect(config: &ClineConfig, home: PathBuf, owner: CredentialOwner) -> Option<AccountRef> {
    let shown = home.display().to_string();
    match account_at(config, home, owner) {
        Ok(account) => account,
        Err(error) => {
            tracing::warn!(dir = %shown, %error, "skipping Cline home");
            None
        }
    }
}

fn account_at(
    config: &ClineConfig,
    home: PathBuf,
    owner: CredentialOwner,
) -> Result<Option<AccountRef>, ProviderError> {
    let Some(text) = read_file(&config.providers_file(&home, owner))? else {
        return Ok(None);
    };
    let credentials = match parse_credentials(&text) {
        Ok(credentials) => credentials,
        Err(ProviderError::NotSignedIn) => return Ok(None),
        Err(error) => return Err(error),
    };
    Ok(Some(AccountRef {
        id: credentials.account_id(),
        provider: super::ID,
        home,
        owner,
    }))
}
