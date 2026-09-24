use std::ffi::OsString;
use std::path::{Path, PathBuf};

use headroom_core::account::CredentialOwner;
use headroom_core::provider::ProviderError;

use crate::paths::HeadroomDirs;

pub const DEFAULT_API_BASE: &str = "https://api.cline.bot";
const DATA_DIR: &str = "data";
const PROVIDERS_FILE: [&str; 2] = ["settings", "providers.json"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClineConfig {
    pub home: PathBuf,
    /// Value of `CLINE_DIR`.
    pub cline_dir: Option<PathBuf>,
    /// Value of `CLINE_DATA_DIR`.
    pub data_dir: Option<PathBuf>,
    pub headroom: HeadroomDirs,
    pub api_base: String,
}

impl ClineConfig {
    #[must_use]
    pub fn for_home(home: PathBuf) -> ClineConfig {
        ClineConfig {
            cline_dir: None,
            data_dir: None,
            headroom: HeadroomDirs::for_home(&home),
            api_base: DEFAULT_API_BASE.to_owned(),
            home,
        }
    }

    pub fn from_env() -> Result<ClineConfig, ProviderError> {
        let home = dirs::home_dir()
            .ok_or_else(|| ProviderError::LocalData("home directory not found".to_owned()))?;
        Ok(ClineConfig::from_vars(home, |name| std::env::var_os(name)))
    }

    #[must_use]
    pub fn from_vars(home: PathBuf, var: impl Fn(&str) -> Option<OsString>) -> ClineConfig {
        let absolute_var = |name: &str| {
            var(name)
                .map(PathBuf::from)
                .filter(|path| path.is_absolute())
        };
        let defaults = ClineConfig::for_home(home);
        ClineConfig {
            cline_dir: absolute_var("CLINE_DIR"),
            data_dir: absolute_var("CLINE_DATA_DIR"),
            headroom: HeadroomDirs::from_vars(&defaults.home, &var),
            ..defaults
        }
    }

    pub(super) fn cli_home(&self) -> PathBuf {
        self.cline_dir
            .clone()
            .unwrap_or_else(|| self.home.join(".cline"))
    }

    pub(super) fn headroom_accounts_dir(&self) -> PathBuf {
        self.headroom.accounts(super::ID.as_str())
    }

    pub(super) fn providers_file(&self, home: &Path, owner: CredentialOwner) -> PathBuf {
        let data_dir = match (owner, &self.data_dir) {
            (CredentialOwner::Cli, Some(data_dir)) => data_dir.clone(),
            _ => home.join(DATA_DIR),
        };
        PROVIDERS_FILE
            .iter()
            .fold(data_dir, |path, part| path.join(part))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    fn config_with(vars: &[(&str, &str)]) -> ClineConfig {
        let vars: HashMap<String, OsString> = vars
            .iter()
            .map(|(name, value)| ((*name).to_owned(), OsString::from(value)))
            .collect();
        ClineConfig::from_vars(PathBuf::from("/home/u"), |name| vars.get(name).cloned())
    }

    #[test]
    fn defaults_follow_home() {
        let config = config_with(&[]);
        assert_eq!(config.cli_home(), PathBuf::from("/home/u/.cline"));
        assert_eq!(
            config.providers_file(&config.cli_home(), CredentialOwner::Cli),
            PathBuf::from("/home/u/.cline/data/settings/providers.json")
        );
        assert_eq!(
            config.headroom_accounts_dir(),
            PathBuf::from("/home/u/.local/share/headroom/accounts/cline")
        );
        assert_eq!(config.api_base, DEFAULT_API_BASE);
    }

    #[test]
    fn environment_overrides_apply_to_the_cli_home_only() {
        let config = config_with(&[
            ("CLINE_DIR", "/work/cline"),
            ("CLINE_DATA_DIR", "/work/cline-data"),
            ("XDG_DATA_HOME", "/data"),
        ]);
        assert_eq!(config.cli_home(), PathBuf::from("/work/cline"));
        assert_eq!(
            config.providers_file(&config.cli_home(), CredentialOwner::Cli),
            PathBuf::from("/work/cline-data/settings/providers.json")
        );
        let owned = Path::new("/data/headroom/accounts/cline/1");
        assert_eq!(
            config.providers_file(owned, CredentialOwner::Headroom),
            owned.join("data/settings/providers.json")
        );
    }

    #[test]
    fn empty_or_relative_values_are_ignored() {
        let config = config_with(&[("CLINE_DIR", ""), ("CLINE_DATA_DIR", "rel")]);
        assert_eq!(config.cli_home(), PathBuf::from("/home/u/.cline"));
        assert_eq!(config.data_dir, None);
    }
}
