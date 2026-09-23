use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use headroom_core::account::{AccountRef, CredentialOwner, ProviderKind};

use super::auth::CREDENTIALS_FILE;
use super::config::ClaudeConfig;
use super::identity::{canonical, load_identity};

struct Candidate {
    dir: PathBuf,
    owner: CredentialOwner,
    needs_credentials: bool,
}

pub(super) fn discover_accounts(config: &ClaudeConfig) -> Vec<AccountRef> {
    let mut seen = BTreeSet::new();
    candidates(config)
        .into_iter()
        .filter_map(|candidate| inspect(&config.home, candidate))
        .filter(|account| seen.insert(account.id.clone()))
        .collect()
}

fn candidates(config: &ClaudeConfig) -> Vec<Candidate> {
    let cli_dir = config.cli_dir();
    let mut list = vec![Candidate {
        dir: cli_dir.clone(),
        owner: CredentialOwner::Cli,
        needs_credentials: false,
    }];
    list.extend(
        scanned_dirs(config, &cli_dir)
            .into_iter()
            .map(|dir| Candidate {
                dir,
                owner: CredentialOwner::Cli,
                needs_credentials: true,
            }),
    );
    list.extend(
        child_dirs(&config.headroom_accounts_dir(), |_| true)
            .into_iter()
            .map(|dir| Candidate {
                dir,
                owner: CredentialOwner::Headroom,
                needs_credentials: true,
            }),
    );
    list
}

fn scanned_dirs(config: &ClaudeConfig, cli_dir: &Path) -> Vec<PathBuf> {
    let mut dirs = child_dirs(&config.home, is_hidden);
    dirs.extend(child_dirs(&config.xdg_config_home, |_| true));
    let excluded = canonical(cli_dir);
    dirs.retain(|dir| canonical(dir) != excluded);
    dirs
}

fn child_dirs(parent: &Path, keep: fn(&Path) -> bool) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(parent) else {
        return Vec::new();
    };
    let mut dirs: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| keep(path) && path.is_dir())
        .collect();
    dirs.sort();
    dirs
}

fn is_hidden(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.starts_with('.'))
}

fn inspect(home: &Path, candidate: Candidate) -> Option<AccountRef> {
    if candidate.needs_credentials && !candidate.dir.join(CREDENTIALS_FILE).is_file() {
        return None;
    }
    let identity = match load_identity(home, &candidate.dir) {
        Ok(identity) => identity?,
        Err(error) => {
            tracing::warn!(dir = %candidate.dir.display(), %error, "skipping Claude config dir");
            return None;
        }
    };
    Some(AccountRef {
        id: identity.account_id(),
        provider: ProviderKind::Claude,
        home: candidate.dir,
        owner: candidate.owner,
    })
}

#[cfg(test)]
#[path = "accounts_tests.rs"]
mod tests;
