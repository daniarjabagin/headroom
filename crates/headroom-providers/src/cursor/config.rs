use std::ffi::OsString;
use std::path::{Path, PathBuf};

use headroom_core::provider::ProviderError;

use crate::paths::{Os, app_config_dir, xdg_config_home};

pub const DEFAULT_API_BASE: &str = "https://api2.cursor.sh";
pub const DEFAULT_WEB_BASE: &str = "https://cursor.com";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CursorConfig {
    /// Where the Cursor agent CLI keeps `cursor/auth.json`.
    pub xdg_config_home: PathBuf,
    /// Where the Cursor app keeps its settings: `$XDG_CONFIG_HOME` or Application Support.
    pub app_config_dir: PathBuf,
    /// Connect RPC host (`api2.cursor.sh`).
    pub api_base: String,
    /// Dashboard host whose `/api/*` endpoints take the session cookie.
    pub web_base: String,
}

impl CursorConfig {
    #[must_use]
    pub fn for_home(home: &Path) -> CursorConfig {
        CursorConfig {
            xdg_config_home: xdg_config_home(home, |_| None),
            app_config_dir: app_config_dir(Os::current(), home, |_| None),
            api_base: DEFAULT_API_BASE.to_owned(),
            web_base: DEFAULT_WEB_BASE.to_owned(),
        }
    }

    pub fn from_env() -> Result<CursorConfig, ProviderError> {
        let home = dirs::home_dir()
            .ok_or_else(|| ProviderError::LocalData("home directory not found".to_owned()))?;
        Ok(CursorConfig::from_vars(&home, |name| {
            std::env::var_os(name)
        }))
    }

    #[must_use]
    pub fn from_vars(home: &Path, var: impl Fn(&str) -> Option<OsString>) -> CursorConfig {
        CursorConfig {
            xdg_config_home: xdg_config_home(home, &var),
            app_config_dir: app_config_dir(Os::current(), home, &var),
            ..CursorConfig::for_home(home)
        }
    }

    pub(super) fn ide_dir(&self) -> PathBuf {
        self.app_config_dir.join("Cursor")
    }

    pub(super) fn state_db(&self) -> PathBuf {
        self.ide_dir().join("User/globalStorage/state.vscdb")
    }

    pub(super) fn agent_dir(&self) -> PathBuf {
        self.xdg_config_home.join("cursor")
    }

    pub(super) fn agent_auth_file(&self) -> PathBuf {
        self.agent_dir().join("auth.json")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_follow_home_by_default() {
        let config = CursorConfig::from_vars(Path::new("/home/u"), |_| None);
        assert_eq!(
            config.state_db(),
            PathBuf::from("/home/u/.config/Cursor/User/globalStorage/state.vscdb")
        );
        assert_eq!(
            config.agent_auth_file(),
            PathBuf::from("/home/u/.config/cursor/auth.json")
        );
        assert_eq!(config.api_base, DEFAULT_API_BASE);
        assert_eq!(config.web_base, DEFAULT_WEB_BASE);
    }

    #[test]
    fn absolute_xdg_config_home_wins() {
        let config = CursorConfig::from_vars(Path::new("/home/u"), |name| {
            (name == "XDG_CONFIG_HOME").then(|| OsString::from("/cfg"))
        });
        assert_eq!(config.ide_dir(), PathBuf::from("/cfg/Cursor"));
        assert_eq!(config.agent_dir(), PathBuf::from("/cfg/cursor"));
    }

    #[test]
    fn relative_xdg_config_home_is_ignored() {
        let config = CursorConfig::from_vars(Path::new("/home/u"), |name| {
            (name == "XDG_CONFIG_HOME").then(|| OsString::from("rel"))
        });
        assert_eq!(config.xdg_config_home, PathBuf::from("/home/u/.config"));
    }
}
