use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use headroom_core::account::CredentialOwner;
use headroom_core::provider::ProviderError;

use crate::homes::unique_dirs;
use crate::paths::HeadroomDirs;

pub const DEFAULT_API_BASE: &str = "https://cli-chat-proxy.grok.com/v1";
pub const DEFAULT_ISSUER: &str = "https://auth.x.ai";
pub(super) const SESSIONS_DIR: &str = "sessions";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GrokConfig {
    pub home: PathBuf,
    /// Value of `GROK_HOME`.
    pub grok_home: Option<PathBuf>,
    pub headroom: HeadroomDirs,
    pub api_base: String,
    /// The only OIDC issuer whose tokens Headroom refreshes in homes it owns.
    pub issuer: String,
}

impl GrokConfig {
    #[must_use]
    pub fn for_home(home: PathBuf) -> GrokConfig {
        GrokConfig {
            grok_home: None,
            headroom: HeadroomDirs::for_home(&home),
            api_base: DEFAULT_API_BASE.to_owned(),
            issuer: DEFAULT_ISSUER.to_owned(),
            home,
        }
    }

    pub fn from_env() -> Result<GrokConfig, ProviderError> {
        let home = dirs::home_dir()
            .ok_or_else(|| ProviderError::LocalData("home directory not found".to_owned()))?;
        Ok(GrokConfig::from_vars(home, |name| std::env::var_os(name)))
    }

    #[must_use]
    pub fn from_vars(home: PathBuf, var: impl Fn(&str) -> Option<OsString>) -> GrokConfig {
        let text_var = |name: &str| {
            var(name)
                .and_then(|value| value.into_string().ok())
                .map(|value| value.trim().to_owned())
                .filter(|value| !value.is_empty())
        };
        let defaults = GrokConfig::for_home(home);
        GrokConfig {
            grok_home: text_var("GROK_HOME").map(|raw| defaults.expand_tilde(&raw)),
            headroom: HeadroomDirs::from_vars(&defaults.home, &var),
            api_base: text_var("GROK_CLI_CHAT_PROXY_BASE_URL")
                .unwrap_or_else(|| defaults.api_base.clone()),
            ..defaults
        }
    }

    pub(super) fn cli_dir(&self) -> PathBuf {
        self.grok_home
            .clone()
            .unwrap_or_else(|| self.home.join(".grok"))
    }

    pub(super) fn headroom_homes(&self) -> Result<Vec<PathBuf>, ProviderError> {
        subdirectories(&self.headroom.accounts(super::ID.as_str()))
    }

    pub(super) fn homes(&self) -> Result<Vec<(PathBuf, CredentialOwner)>, ProviderError> {
        let cli = (self.cli_dir(), CredentialOwner::Cli);
        let headroom = self
            .headroom_homes()?
            .into_iter()
            .map(|home| (home, CredentialOwner::Headroom));
        Ok(headroom.chain(std::iter::once(cli)).collect())
    }

    pub(super) fn usage_homes(&self) -> Result<Vec<PathBuf>, ProviderError> {
        let homes = std::iter::once(self.cli_dir()).chain(self.headroom_homes()?);
        Ok(unique_dirs(
            homes.filter(|home| home.join(SESSIONS_DIR).is_dir()),
        ))
    }

    fn expand_tilde(&self, raw: &str) -> PathBuf {
        if raw == "~" {
            return self.home.clone();
        }
        match raw.strip_prefix("~/") {
            Some(rest) => self.home.join(rest),
            None => PathBuf::from(raw),
        }
    }
}

fn subdirectories(root: &Path) -> Result<Vec<PathBuf>, ProviderError> {
    let entries = match fs::read_dir(root) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(read_error(root, &error)),
    };
    let mut homes = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|error| read_error(root, &error))?;
        if entry.file_type().is_ok_and(|kind| kind.is_dir()) {
            homes.push(entry.path());
        }
    }
    homes.sort();
    Ok(homes)
}

fn read_error(path: &Path, error: &io::Error) -> ProviderError {
    ProviderError::LocalData(format!("cannot read {}: {error}", path.display()))
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    fn config_with(vars: &[(&str, &str)]) -> GrokConfig {
        let vars: HashMap<String, OsString> = vars
            .iter()
            .map(|(name, value)| ((*name).to_owned(), OsString::from(value)))
            .collect();
        GrokConfig::from_vars(PathBuf::from("/home/u"), |name| vars.get(name).cloned())
    }

    #[test]
    fn defaults_follow_the_cli() {
        let config = config_with(&[]);
        assert_eq!(config.cli_dir(), PathBuf::from("/home/u/.grok"));
        assert_eq!(config.api_base, DEFAULT_API_BASE);
        assert_eq!(
            config.headroom.accounts("grok"),
            PathBuf::from("/home/u/.local/share/headroom/accounts/grok")
        );
    }

    #[test]
    fn grok_home_and_proxy_override_are_honoured() {
        let config = config_with(&[
            ("GROK_HOME", " ~/work/grok "),
            (
                "GROK_CLI_CHAT_PROXY_BASE_URL",
                "https://proxy.example.com/v1",
            ),
            ("XDG_DATA_HOME", "relative/ignored"),
        ]);
        assert_eq!(config.cli_dir(), PathBuf::from("/home/u/work/grok"));
        assert_eq!(config.api_base, "https://proxy.example.com/v1");
        assert_eq!(
            config.headroom.accounts("grok"),
            PathBuf::from("/home/u/.local/share/headroom/accounts/grok")
        );
        assert_eq!(
            config_with(&[("GROK_HOME", "/srv/grok")]).cli_dir(),
            PathBuf::from("/srv/grok")
        );
        assert_eq!(
            config_with(&[("GROK_HOME", "  ")]).cli_dir(),
            PathBuf::from("/home/u/.grok")
        );
    }

    #[test]
    fn homes_list_headroom_accounts_before_the_cli_home() {
        let root = tempfile::tempdir().unwrap();
        let accounts = root.path().join("data/headroom/accounts/grok");
        fs::create_dir_all(accounts.join("b")).unwrap();
        fs::create_dir_all(accounts.join("a")).unwrap();
        fs::write(accounts.join("stray.txt"), "x").unwrap();
        let config = GrokConfig {
            headroom: HeadroomDirs {
                data: root.path().join("data/headroom"),
            },
            ..GrokConfig::for_home(root.path().join("home"))
        };
        assert_eq!(
            config.homes().unwrap(),
            [
                (accounts.join("a"), CredentialOwner::Headroom),
                (accounts.join("b"), CredentialOwner::Headroom),
                (root.path().join("home/.grok"), CredentialOwner::Cli),
            ]
        );
    }

    #[test]
    fn usage_homes_need_a_sessions_directory() {
        let root = tempfile::tempdir().unwrap();
        let accounts = root.path().join("data/headroom/accounts/grok");
        fs::create_dir_all(root.path().join("home/.grok/sessions")).unwrap();
        fs::create_dir_all(accounts.join("a/sessions")).unwrap();
        fs::create_dir_all(accounts.join("b")).unwrap();
        let config = GrokConfig {
            headroom: HeadroomDirs {
                data: root.path().join("data/headroom"),
            },
            ..GrokConfig::for_home(root.path().join("home"))
        };
        assert_eq!(
            config.usage_homes().unwrap(),
            [root.path().join("home/.grok"), accounts.join("a")]
        );
    }
}
