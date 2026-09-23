use std::ffi::OsString;
use std::path::{Path, PathBuf};

use headroom_core::provider::ProviderError;

pub const GLOBAL_API_BASE: &str = "https://www.minimax.io";

const ACCOUNTS_DIR: &str = "headroom/accounts/minimax";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MiniMaxConfig {
    /// Headroom-owned homes, `$XDG_DATA_HOME/headroom/accounts/minimax`.
    pub accounts_dir: PathBuf,
    pub api_base: String,
}

impl MiniMaxConfig {
    #[must_use]
    pub fn for_home(home: &Path) -> MiniMaxConfig {
        MiniMaxConfig {
            accounts_dir: home.join(".local/share").join(ACCOUNTS_DIR),
            api_base: GLOBAL_API_BASE.to_owned(),
        }
    }

    pub fn from_env() -> Result<MiniMaxConfig, ProviderError> {
        let home = dirs::home_dir()
            .ok_or_else(|| ProviderError::LocalData("home directory not found".to_owned()))?;
        Ok(MiniMaxConfig::from_vars(&home, |name| {
            std::env::var_os(name)
        }))
    }

    #[must_use]
    pub fn from_vars(home: &Path, var: impl Fn(&str) -> Option<OsString>) -> MiniMaxConfig {
        let defaults = MiniMaxConfig::for_home(home);
        let data_home = var("XDG_DATA_HOME")
            .map(PathBuf::from)
            .filter(|path| path.is_absolute());
        MiniMaxConfig {
            accounts_dir: data_home.map_or(defaults.accounts_dir, |data| data.join(ACCOUNTS_DIR)),
            ..defaults
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accounts_live_under_the_data_home() {
        let home = Path::new("/home/u");
        let config = MiniMaxConfig::from_vars(home, |_| None);
        assert_eq!(config, MiniMaxConfig::for_home(home));
        assert_eq!(
            config.accounts_dir,
            PathBuf::from("/home/u/.local/share/headroom/accounts/minimax")
        );
        assert_eq!(config.api_base, GLOBAL_API_BASE);
        let moved = MiniMaxConfig::from_vars(home, |name| {
            (name == "XDG_DATA_HOME").then(|| OsString::from("/data"))
        });
        assert_eq!(
            moved.accounts_dir,
            PathBuf::from("/data/headroom/accounts/minimax")
        );
        let relative = MiniMaxConfig::from_vars(home, |_| Some(OsString::from("rel")));
        assert_eq!(relative, config);
    }
}
