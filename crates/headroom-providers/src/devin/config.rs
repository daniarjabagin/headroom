use std::ffi::OsString;
use std::path::{Path, PathBuf};

use headroom_core::provider::ProviderError;

pub const DEFAULT_API_BASE: &str = "https://server.codeium.com";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DevinConfig {
    pub xdg_data_home: PathBuf,
    pub xdg_config_home: PathBuf,
    /// Used when the credentials name no `api_server_url`.
    pub api_base: String,
}

impl DevinConfig {
    #[must_use]
    pub fn for_home(home: &Path) -> DevinConfig {
        DevinConfig {
            xdg_data_home: home.join(".local/share"),
            xdg_config_home: home.join(".config"),
            api_base: DEFAULT_API_BASE.to_owned(),
        }
    }

    pub fn from_env() -> Result<DevinConfig, ProviderError> {
        let home = dirs::home_dir()
            .ok_or_else(|| ProviderError::LocalData("home directory not found".to_owned()))?;
        Ok(DevinConfig::from_vars(&home, |name| std::env::var_os(name)))
    }

    #[must_use]
    pub fn from_vars(home: &Path, var: impl Fn(&str) -> Option<OsString>) -> DevinConfig {
        let absolute_var = |name: &str| {
            var(name)
                .map(PathBuf::from)
                .filter(|path| path.is_absolute())
        };
        let defaults = DevinConfig::for_home(home);
        DevinConfig {
            xdg_data_home: absolute_var("XDG_DATA_HOME").unwrap_or(defaults.xdg_data_home),
            xdg_config_home: absolute_var("XDG_CONFIG_HOME").unwrap_or(defaults.xdg_config_home),
            ..defaults
        }
    }

    pub(super) fn cli_dir(&self) -> PathBuf {
        self.xdg_data_home.join(super::DATA_SUBDIR)
    }

    pub(super) fn app_state_dir(&self) -> PathBuf {
        self.xdg_config_home.join("Devin/User/globalStorage")
    }

    pub(super) fn headroom_accounts_dir(&self) -> PathBuf {
        self.xdg_data_home.join("headroom/accounts/devin")
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    fn config_with(vars: &[(&str, &str)]) -> DevinConfig {
        let vars: HashMap<String, OsString> = vars
            .iter()
            .map(|(name, value)| ((*name).to_owned(), OsString::from(value)))
            .collect();
        DevinConfig::from_vars(Path::new("/home/u"), |name| vars.get(name).cloned())
    }

    #[test]
    fn defaults_follow_home() {
        let config = config_with(&[]);
        assert_eq!(
            config.cli_dir(),
            PathBuf::from("/home/u/.local/share/devin")
        );
        assert_eq!(
            config.app_state_dir(),
            PathBuf::from("/home/u/.config/Devin/User/globalStorage")
        );
        assert_eq!(
            config.headroom_accounts_dir(),
            PathBuf::from("/home/u/.local/share/headroom/accounts/devin")
        );
        assert_eq!(config.api_base, DEFAULT_API_BASE);
    }

    #[test]
    fn absolute_xdg_dirs_override_and_relative_ones_are_ignored() {
        let config = config_with(&[("XDG_DATA_HOME", "/data"), ("XDG_CONFIG_HOME", "cfg")]);
        assert_eq!(config.cli_dir(), PathBuf::from("/data/devin"));
        assert_eq!(config.xdg_config_home, PathBuf::from("/home/u/.config"));
    }
}
