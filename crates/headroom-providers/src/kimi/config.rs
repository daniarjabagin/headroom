use std::ffi::OsString;
use std::path::{Path, PathBuf};

use headroom_core::provider::ProviderError;

use crate::paths::HeadroomDirs;

pub const DEFAULT_API_BASE: &str = "https://api.kimi.com/coding/v1";
pub const DEFAULT_OAUTH_HOST: &str = "https://auth.kimi.com";

const SHARE_DIR_VAR: &str = "KIMI_SHARE_DIR";
const API_BASE_VAR: &str = "KIMI_CODE_BASE_URL";
const OAUTH_HOST_VARS: [&str; 2] = ["KIMI_CODE_OAUTH_HOST", "KIMI_OAUTH_HOST"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KimiConfig {
    /// The `kimi` CLI's own share directory (`$KIMI_SHARE_DIR` or `~/.kimi`).
    pub share_dir: PathBuf,
    /// Headroom-owned homes, `<Headroom data dir>/accounts/kimi`.
    pub accounts_dir: PathBuf,
    pub api_base: String,
    pub oauth_host: String,
}

impl KimiConfig {
    #[must_use]
    pub fn for_home(home: &Path) -> KimiConfig {
        KimiConfig {
            share_dir: home.join(".kimi"),
            accounts_dir: HeadroomDirs::for_home(home).accounts(super::ID.as_str()),
            api_base: DEFAULT_API_BASE.to_owned(),
            oauth_host: DEFAULT_OAUTH_HOST.to_owned(),
        }
    }

    pub fn from_env() -> Result<KimiConfig, ProviderError> {
        let home = dirs::home_dir()
            .ok_or_else(|| ProviderError::LocalData("home directory not found".to_owned()))?;
        Ok(KimiConfig::from_vars(&home, |name| std::env::var_os(name)))
    }

    #[must_use]
    pub fn from_vars(home: &Path, var: impl Fn(&str) -> Option<OsString>) -> KimiConfig {
        let text = |name: &str| {
            var(name)
                .and_then(|value| value.into_string().ok())
                .map(|value| value.trim().to_owned())
                .filter(|value| !value.is_empty())
        };
        let absolute = |name: &str| text(name).map(PathBuf::from).filter(|p| p.is_absolute());
        let defaults = KimiConfig::for_home(home);
        KimiConfig {
            share_dir: absolute(SHARE_DIR_VAR).unwrap_or(defaults.share_dir),
            accounts_dir: HeadroomDirs::from_vars(home, &var).accounts(super::ID.as_str()),
            api_base: text(API_BASE_VAR).unwrap_or(defaults.api_base),
            oauth_host: OAUTH_HOST_VARS
                .into_iter()
                .find_map(text)
                .unwrap_or(defaults.oauth_host),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    fn config_with(vars: &[(&str, &str)]) -> KimiConfig {
        let vars: HashMap<String, OsString> = vars
            .iter()
            .map(|(name, value)| ((*name).to_owned(), OsString::from(value)))
            .collect();
        KimiConfig::from_vars(Path::new("/home/u"), |name| vars.get(name).cloned())
    }

    #[test]
    fn defaults_follow_home() {
        let config = config_with(&[]);
        assert_eq!(config, KimiConfig::for_home(Path::new("/home/u")));
        assert_eq!(config.share_dir, PathBuf::from("/home/u/.kimi"));
        assert_eq!(
            config.accounts_dir,
            PathBuf::from("/home/u/.local/share/headroom/accounts/kimi")
        );
        assert_eq!(config.api_base, DEFAULT_API_BASE);
        assert_eq!(config.oauth_host, DEFAULT_OAUTH_HOST);
    }

    #[test]
    fn the_cli_variables_override_defaults() {
        let config = config_with(&[
            ("KIMI_SHARE_DIR", "/work/kimi"),
            ("XDG_DATA_HOME", "/data"),
            ("KIMI_CODE_BASE_URL", "https://kimi.example/coding/v1"),
            ("KIMI_OAUTH_HOST", "https://auth.example"),
        ]);
        assert_eq!(config.share_dir, PathBuf::from("/work/kimi"));
        assert_eq!(
            config.accounts_dir,
            PathBuf::from("/data/headroom/accounts/kimi")
        );
        assert_eq!(config.api_base, "https://kimi.example/coding/v1");
        assert_eq!(config.oauth_host, "https://auth.example");
    }

    #[test]
    fn empty_or_relative_values_are_ignored() {
        let config = config_with(&[
            ("KIMI_SHARE_DIR", "rel"),
            ("XDG_DATA_HOME", " "),
            ("KIMI_CODE_OAUTH_HOST", ""),
        ]);
        assert_eq!(config, KimiConfig::for_home(Path::new("/home/u")));
    }
}
