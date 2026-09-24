use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use headroom_core::account::CredentialOwner;
use headroom_core::provider::ProviderError;

use super::local_usage::has_logs;
use crate::homes::unique_dirs;
use crate::keychain::Security;
use crate::paths::{HeadroomDirs, Os};

const CODEX_HOME_VAR: &str = "CODEX_HOME";
const DEFAULT_HOME_DIR: &str = ".codex";

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CodexEnvironment {
    pub codex_home: Option<String>,
    pub home_dir: Option<PathBuf>,
    pub headroom: Option<HeadroomDirs>,
    /// Read when `cli_auth_credentials_store` keeps the sign-in in the macOS Keychain.
    pub keychain: Option<Security>,
}

impl CodexEnvironment {
    #[must_use]
    pub fn from_process() -> CodexEnvironment {
        CodexEnvironment {
            codex_home: std::env::var(CODEX_HOME_VAR).ok(),
            home_dir: dirs::home_dir(),
            headroom: HeadroomDirs::from_process(),
            keychain: (Os::current() == Os::MacOs).then(Security::system),
        }
    }

    pub(super) fn cli_home(&self) -> Option<PathBuf> {
        match self.codex_home.as_deref().map(str::trim) {
            Some(raw) if !raw.is_empty() => self.expand_tilde(raw),
            _ => self
                .home_dir
                .as_ref()
                .map(|home| home.join(DEFAULT_HOME_DIR)),
        }
    }

    pub(super) fn headroom_homes(&self) -> Result<Vec<PathBuf>, ProviderError> {
        let Some(headroom) = &self.headroom else {
            return Ok(Vec::new());
        };
        subdirectories(&headroom.accounts(super::ID.as_str()))
    }

    pub(super) fn usage_homes(&self) -> Result<Vec<PathBuf>, ProviderError> {
        let homes = self.cli_home().into_iter().chain(self.headroom_homes()?);
        Ok(unique_dirs(homes.filter(|home| has_logs(home))))
    }

    pub(super) fn homes(&self) -> Result<Vec<(PathBuf, CredentialOwner)>, ProviderError> {
        let cli = self
            .cli_home()
            .into_iter()
            .map(|home| (home, CredentialOwner::Cli));
        let headroom = self
            .headroom_homes()?
            .into_iter()
            .map(|home| (home, CredentialOwner::Headroom));
        Ok(cli.chain(headroom).collect())
    }

    fn expand_tilde(&self, raw: &str) -> Option<PathBuf> {
        if raw == "~" {
            return self.home_dir.clone();
        }
        match raw.strip_prefix("~/") {
            Some(rest) => self.home_dir.as_ref().map(|home| home.join(rest)),
            None => Some(PathBuf::from(raw)),
        }
    }
}

fn subdirectories(root: &Path) -> Result<Vec<PathBuf>, ProviderError> {
    let entries = match fs::read_dir(root) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(local_error(root, &error)),
    };
    let mut homes = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|error| local_error(root, &error))?;
        if entry.file_type().is_ok_and(|kind| kind.is_dir()) {
            homes.push(entry.path());
        }
    }
    homes.sort();
    Ok(homes)
}

fn local_error(path: &Path, error: &io::Error) -> ProviderError {
    ProviderError::LocalData(format!("cannot read {}: {error}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env(codex_home: Option<&str>) -> CodexEnvironment {
        CodexEnvironment {
            codex_home: codex_home.map(str::to_owned),
            home_dir: Some(PathBuf::from("/home/user")),
            ..CodexEnvironment::default()
        }
    }

    #[test]
    fn default_home_is_dot_codex() {
        assert_eq!(
            env(None).cli_home(),
            Some(PathBuf::from("/home/user/.codex"))
        );
        assert_eq!(
            env(Some("  ")).cli_home(),
            Some(PathBuf::from("/home/user/.codex"))
        );
    }

    #[test]
    fn codex_home_is_a_single_expanded_path() {
        assert_eq!(
            env(Some("~/work/codex")).cli_home(),
            Some(PathBuf::from("/home/user/work/codex"))
        );
        assert_eq!(env(Some("~")).cli_home(), Some(PathBuf::from("/home/user")));
        assert_eq!(
            env(Some(" /srv/a,/srv/b ")).cli_home(),
            Some(PathBuf::from("/srv/a,/srv/b"))
        );
    }

    #[test]
    fn headroom_homes_are_account_subdirectories() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("headroom/accounts/codex");
        fs::create_dir_all(root.join("b-uuid")).unwrap();
        fs::create_dir_all(root.join("a-uuid")).unwrap();
        fs::write(root.join("stray.txt"), "x").unwrap();
        let environment = CodexEnvironment {
            headroom: Some(HeadroomDirs {
                data: dir.path().join("headroom"),
            }),
            ..env(None)
        };
        assert_eq!(
            environment.homes().unwrap(),
            [
                (PathBuf::from("/home/user/.codex"), CredentialOwner::Cli),
                (root.join("a-uuid"), CredentialOwner::Headroom),
                (root.join("b-uuid"), CredentialOwner::Headroom),
            ]
        );
    }

    #[test]
    fn missing_data_dir_has_no_headroom_homes() {
        let dir = tempfile::tempdir().unwrap();
        let environment = CodexEnvironment {
            headroom: Some(HeadroomDirs {
                data: dir.path().join("absent"),
            }),
            ..env(None)
        };
        assert!(environment.headroom_homes().unwrap().is_empty());
    }
}
