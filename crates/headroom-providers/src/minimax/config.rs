use std::ffi::OsString;
use std::path::{Path, PathBuf};

use headroom_core::provider::ProviderError;

use crate::paths::HeadroomDirs;

pub const GLOBAL_API_BASE: &str = "https://www.minimax.io";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MiniMaxConfig {
    /// Headroom-owned homes, `<Headroom data dir>/accounts/minimax`.
    pub accounts_dir: PathBuf,
    pub api_base: String,
}

impl MiniMaxConfig {
    #[must_use]
    pub fn for_home(home: &Path) -> MiniMaxConfig {
        MiniMaxConfig {
            accounts_dir: accounts_dir(&HeadroomDirs::for_home(home)),
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
        MiniMaxConfig {
            accounts_dir: accounts_dir(&HeadroomDirs::from_vars(home, var)),
            ..MiniMaxConfig::for_home(home)
        }
    }
}

fn accounts_dir(dirs: &HeadroomDirs) -> PathBuf {
    dirs.accounts(super::ID.as_str())
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
