use std::ffi::OsString;
use std::path::{Path, PathBuf};

use headroom_core::provider::ProviderError;

pub const DEFAULT_API_BASE: &str = "https://opencode.ai";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenCodeConfig {
    pub data_dir: PathBuf,
    pub accounts_dir: PathBuf,
    pub api_base: String,
}

impl OpenCodeConfig {
    #[must_use]
    pub fn for_home(home: &Path) -> OpenCodeConfig {
        OpenCodeConfig::from_data_home(&home.join(".local/share"))
    }

    pub fn from_env() -> Result<OpenCodeConfig, ProviderError> {
        let home = dirs::home_dir()
            .ok_or_else(|| ProviderError::LocalData("home directory not found".to_owned()))?;
        Ok(OpenCodeConfig::from_vars(&home, |name| {
            std::env::var_os(name)
        }))
    }

    #[must_use]
    pub fn from_vars(home: &Path, var: impl Fn(&str) -> Option<OsString>) -> OpenCodeConfig {
        let absolute_var = |name: &str| {
            var(name)
                .map(PathBuf::from)
                .filter(|path| path.is_absolute())
        };
        let data_home = absolute_var("XDG_DATA_HOME").unwrap_or_else(|| home.join(".local/share"));
        let defaults = OpenCodeConfig::from_data_home(&data_home);
        OpenCodeConfig {
            data_dir: absolute_var("OPENCODE_DATA_DIR").unwrap_or(defaults.data_dir),
            ..defaults
        }
    }

    fn from_data_home(data_home: &Path) -> OpenCodeConfig {
        OpenCodeConfig {
            data_dir: data_home.join("opencode"),
            accounts_dir: data_home.join("headroom/accounts/opencode"),
            api_base: DEFAULT_API_BASE.to_owned(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    fn config_with(vars: &[(&str, &str)]) -> OpenCodeConfig {
        let vars: HashMap<String, OsString> = vars
            .iter()
            .map(|(name, value)| ((*name).to_owned(), OsString::from(value)))
            .collect();
        OpenCodeConfig::from_vars(Path::new("/home/u"), |name| vars.get(name).cloned())
    }

    #[test]
    fn defaults_follow_home() {
        let config = config_with(&[]);
        assert_eq!(config, OpenCodeConfig::for_home(Path::new("/home/u")));
        assert_eq!(
            config.data_dir,
            PathBuf::from("/home/u/.local/share/opencode")
        );
        assert_eq!(
            config.accounts_dir,
            PathBuf::from("/home/u/.local/share/headroom/accounts/opencode")
        );
        assert_eq!(config.api_base, DEFAULT_API_BASE);
    }

    #[test]
    fn xdg_data_home_and_data_dir_override() {
        let config = config_with(&[("XDG_DATA_HOME", "/data")]);
        assert_eq!(config.data_dir, PathBuf::from("/data/opencode"));
        assert_eq!(
            config.accounts_dir,
            PathBuf::from("/data/headroom/accounts/opencode")
        );
        let config = config_with(&[("XDG_DATA_HOME", "/data"), ("OPENCODE_DATA_DIR", "/oc")]);
        assert_eq!(config.data_dir, PathBuf::from("/oc"));
        assert_eq!(
            config.accounts_dir,
            PathBuf::from("/data/headroom/accounts/opencode")
        );
    }

    #[test]
    fn empty_or_relative_values_are_ignored() {
        let config = config_with(&[("XDG_DATA_HOME", "rel"), ("OPENCODE_DATA_DIR", "")]);
        assert_eq!(config, OpenCodeConfig::for_home(Path::new("/home/u")));
    }
}
