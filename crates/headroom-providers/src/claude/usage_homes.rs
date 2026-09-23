use std::path::{Path, PathBuf};

use super::accounts::{child_dirs, scanned_dirs};
use super::auth::CREDENTIALS_FILE;
use super::config::ClaudeConfig;
use super::identity::IDENTITY_FILE;
use super::local_usage::PROJECTS_DIR;
use crate::homes::unique_dirs;

pub(super) fn usage_homes(config: &ClaudeConfig) -> Vec<PathBuf> {
    let cli_dir = config.cli_dir();
    let known = [cli_dir.clone(), config.default_dir()];
    let scanned = scanned_dirs(config, &cli_dir)
        .into_iter()
        .filter(|dir| has_config_file(dir));
    let owned = child_dirs(&config.headroom_accounts_dir(), |_| true);
    let candidates = known.into_iter().chain(scanned).chain(owned);
    unique_dirs(candidates.filter(|dir| has_projects(dir)))
}

fn has_projects(dir: &Path) -> bool {
    dir.join(PROJECTS_DIR).is_dir()
}

fn has_config_file(dir: &Path) -> bool {
    dir.join(IDENTITY_FILE).is_file() || dir.join(CREDENTIALS_FILE).is_file()
}

#[cfg(test)]
#[path = "usage_homes_tests.rs"]
mod tests;
