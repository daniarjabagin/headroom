use std::ffi::OsString;
use std::path::PathBuf;

use headroom_core::provider::ProviderError;

use crate::keychain::Security;
use crate::paths::{HeadroomDirs, Os, xdg_config_home};

pub const DEFAULT_API_BASE: &str = "https://api.anthropic.com";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaudeConfig {
    pub home: PathBuf,
    /// Value of `CLAUDE_CONFIG_DIR`.
    pub config_dir: Option<PathBuf>,
    pub xdg_config_home: PathBuf,
    pub headroom: HeadroomDirs,
    pub api_base: String,
    /// Keychain account of Claude Code's sign-in (`$USER`).
    pub user: Option<String>,
    /// Where Claude Code keeps sign-ins on macOS; `None` reads only `.credentials.json`.
    pub keychain: Option<Security>,
}

impl ClaudeConfig {
    #[must_use]
    pub fn for_home(home: PathBuf) -> ClaudeConfig {
        ClaudeConfig {
            config_dir: None,
            xdg_config_home: xdg_config_home(&home, |_| None),
            headroom: HeadroomDirs::for_home(&home),
            api_base: DEFAULT_API_BASE.to_owned(),
            user: None,
            keychain: default_keychain(),
            home,
        }
    }

    pub fn from_env() -> Result<ClaudeConfig, ProviderError> {
        let home = dirs::home_dir()
            .ok_or_else(|| ProviderError::LocalData("home directory not found".to_owned()))?;
        Ok(ClaudeConfig::from_vars(home, |name| std::env::var_os(name)))
    }

    #[must_use]
    pub fn from_vars(home: PathBuf, var: impl Fn(&str) -> Option<OsString>) -> ClaudeConfig {
        let path_var = |name: &str| {
            var(name)
                .map(PathBuf::from)
                .filter(|p| !p.as_os_str().is_empty())
        };
        let user = var("USER")
            .and_then(|user| user.into_string().ok())
            .filter(|user| !user.is_empty());
        ClaudeConfig {
            config_dir: path_var("CLAUDE_CONFIG_DIR"),
            xdg_config_home: xdg_config_home(&home, &var),
            headroom: HeadroomDirs::from_vars(&home, &var),
            user,
            ..ClaudeConfig::for_home(home)
        }
    }

    pub(super) fn cli_dir(&self) -> PathBuf {
        self.config_dir
            .clone()
            .unwrap_or_else(|| self.default_dir())
    }

    pub(super) fn default_dir(&self) -> PathBuf {
        self.home.join(".claude")
    }

    pub(super) fn headroom_accounts_dir(&self) -> PathBuf {
        self.headroom.accounts(super::ID.as_str())
    }
}

fn default_keychain() -> Option<Security> {
    (Os::current() == Os::MacOs).then(Security::system)
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    fn config_with(vars: &[(&str, &str)]) -> ClaudeConfig {
        let vars: HashMap<String, OsString> = vars
            .iter()
            .map(|(name, value)| ((*name).to_owned(), OsString::from(value)))
            .collect();
        ClaudeConfig::from_vars(PathBuf::from("/home/u"), |name| vars.get(name).cloned())
    }

    #[test]
    fn defaults_follow_home() {
        let config = config_with(&[]);
        assert_eq!(config.cli_dir(), PathBuf::from("/home/u/.claude"));
        assert_eq!(config.xdg_config_home, PathBuf::from("/home/u/.config"));
        assert_eq!(
            config.headroom_accounts_dir(),
            PathBuf::from("/home/u/.local/share/headroom/accounts/claude")
        );
        assert_eq!(config.api_base, DEFAULT_API_BASE);
        assert_eq!(config.user, None);
    }

    #[test]
    fn the_keychain_account_is_the_login_user() {
        assert_eq!(config_with(&[("USER", "u")]).user.as_deref(), Some("u"));
        assert_eq!(config_with(&[("USER", "")]).user, None);
    }

    #[test]
    fn environment_overrides_dirs() {
        let config = config_with(&[
            ("CLAUDE_CONFIG_DIR", "/work/claude-alt"),
            ("XDG_CONFIG_HOME", "/cfg"),
            ("XDG_DATA_HOME", "/data"),
        ]);
        assert_eq!(config.cli_dir(), PathBuf::from("/work/claude-alt"));
        assert_eq!(config.xdg_config_home, PathBuf::from("/cfg"));
        assert_eq!(
            config.headroom_accounts_dir(),
            PathBuf::from("/data/headroom/accounts/claude")
        );
    }

    #[test]
    fn empty_or_relative_values_are_ignored() {
        let config = config_with(&[("CLAUDE_CONFIG_DIR", ""), ("XDG_DATA_HOME", "rel")]);
        assert_eq!(config.cli_dir(), PathBuf::from("/home/u/.claude"));
        assert_eq!(
            config.headroom_accounts_dir(),
            PathBuf::from("/home/u/.local/share/headroom/accounts/claude")
        );
    }
}
