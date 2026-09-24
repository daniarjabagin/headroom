use std::ffi::OsString;
use std::path::{Path, PathBuf};

use headroom_core::provider::ProviderError;

use crate::paths::HeadroomDirs;

pub const DEFAULT_API_BASE: &str = "https://api.kilo.ai";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KiloConfig {
    /// The `kilo` CLI's data directory: `$XDG_DATA_HOME/kilo` or `~/.local/share/kilo` on every OS.
    pub data_dir: PathBuf,
    pub accounts_dir: PathBuf,
    pub api_base: String,
}

impl KiloConfig {
    #[must_use]
    pub fn for_home(home: &Path) -> KiloConfig {
        KiloConfig::from_vars(home, |_| None)
    }

    pub fn from_env() -> Result<KiloConfig, ProviderError> {
        let home = dirs::home_dir()
            .ok_or_else(|| ProviderError::LocalData("home directory not found".to_owned()))?;
        Ok(KiloConfig::from_vars(&home, |name| std::env::var_os(name)))
    }

    #[must_use]
    pub fn from_vars(home: &Path, var: impl Fn(&str) -> Option<OsString>) -> KiloConfig {
        let data_home = var("XDG_DATA_HOME")
            .map(PathBuf::from)
            .filter(|path| path.is_absolute())
            .unwrap_or_else(|| home.join(".local/share"));
        KiloConfig {
            data_dir: data_home.join(super::DATA_SUBDIR),
            accounts_dir: HeadroomDirs::from_vars(home, &var).accounts(super::ID.as_str()),
            api_base: DEFAULT_API_BASE.to_owned(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::paths::per_os;

    fn config_with(vars: &[(&str, &str)]) -> KiloConfig {
        let vars: HashMap<String, OsString> = vars
            .iter()
            .map(|(name, value)| ((*name).to_owned(), OsString::from(value)))
            .collect();
        KiloConfig::from_vars(Path::new("/home/u"), |name| vars.get(name).cloned())
    }

    #[test]
    fn defaults_follow_home_on_every_os() {
        let config = config_with(&[]);
        assert_eq!(config, KiloConfig::for_home(Path::new("/home/u")));
        assert_eq!(config.data_dir, PathBuf::from("/home/u/.local/share/kilo"));
        assert_eq!(
            config.accounts_dir,
            per_os(
                "/home/u/.local/share/headroom/accounts/kilo",
                "/home/u/Library/Application Support/Headroom/accounts/kilo",
            )
        );
        assert_eq!(config.api_base, DEFAULT_API_BASE);
    }

    #[test]
    fn an_absolute_xdg_data_home_moves_the_cli_data() {
        let config = config_with(&[("XDG_DATA_HOME", "/data"), ("KILO_API_URL", "https://x")]);
        assert_eq!(config.data_dir, PathBuf::from("/data/kilo"));
        assert_eq!(config.api_base, DEFAULT_API_BASE);
        let relative = config_with(&[("XDG_DATA_HOME", "rel")]);
        assert_eq!(relative, KiloConfig::for_home(Path::new("/home/u")));
    }
}
