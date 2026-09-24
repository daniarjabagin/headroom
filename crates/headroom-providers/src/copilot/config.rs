use std::ffi::OsString;
use std::path::{Path, PathBuf};

use headroom_core::provider::ProviderError;

use crate::paths::HeadroomDirs;

pub const DEFAULT_API_BASE: &str = "https://api.github.com";
const GH_PROGRAM: &str = "gh";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CopilotConfig {
    /// The GitHub CLI's own config dir: `$GH_CONFIG_DIR`, else `$XDG_CONFIG_HOME/gh`.
    pub gh_config_dir: PathBuf,
    pub headroom: HeadroomDirs,
    pub api_base: String,
    /// The `gh` executable asked for account tokens; a bare name is looked up on `PATH`.
    pub gh_program: PathBuf,
}

impl CopilotConfig {
    #[must_use]
    pub fn for_home(home: &Path) -> CopilotConfig {
        CopilotConfig {
            gh_config_dir: home.join(".config/gh"),
            headroom: HeadroomDirs::for_home(home),
            api_base: DEFAULT_API_BASE.to_owned(),
            gh_program: PathBuf::from(GH_PROGRAM),
        }
    }

    pub fn from_env() -> Result<CopilotConfig, ProviderError> {
        let home = dirs::home_dir()
            .ok_or_else(|| ProviderError::LocalData("home directory not found".to_owned()))?;
        Ok(CopilotConfig::from_vars(&home, |name| {
            std::env::var_os(name)
        }))
    }

    #[must_use]
    pub fn from_vars(home: &Path, var: impl Fn(&str) -> Option<OsString>) -> CopilotConfig {
        let path_var = |name: &str| {
            var(name)
                .map(PathBuf::from)
                .filter(|path| !path.as_os_str().is_empty())
        };
        let absolute_var = |name: &str| path_var(name).filter(|path| path.is_absolute());
        let defaults = CopilotConfig::for_home(home);
        let gh_config_dir = path_var("GH_CONFIG_DIR")
            .or_else(|| absolute_var("XDG_CONFIG_HOME").map(|dir| dir.join("gh")))
            .unwrap_or(defaults.gh_config_dir);
        CopilotConfig {
            gh_config_dir,
            headroom: HeadroomDirs::from_vars(home, &var),
            ..defaults
        }
    }

    pub(super) fn headroom_accounts_dir(&self) -> PathBuf {
        self.headroom.accounts(super::ID.as_str())
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    fn config_with(vars: &[(&str, &str)]) -> CopilotConfig {
        let vars: HashMap<String, OsString> = vars
            .iter()
            .map(|(name, value)| ((*name).to_owned(), OsString::from(value)))
            .collect();
        CopilotConfig::from_vars(Path::new("/home/u"), |name| vars.get(name).cloned())
    }

    #[test]
    fn defaults_follow_home() {
        let config = config_with(&[]);
        assert_eq!(config.gh_config_dir, PathBuf::from("/home/u/.config/gh"));
        assert_eq!(
            config.headroom_accounts_dir(),
            PathBuf::from("/home/u/.local/share/headroom/accounts/copilot")
        );
        assert_eq!(config.gh_program, PathBuf::from("gh"));
        assert_eq!(config.api_base, DEFAULT_API_BASE);
    }

    #[test]
    fn gh_config_dir_wins_over_xdg_config_home() {
        let xdg = config_with(&[("XDG_CONFIG_HOME", "/cfg"), ("XDG_DATA_HOME", "/data")]);
        assert_eq!(xdg.gh_config_dir, PathBuf::from("/cfg/gh"));
        assert_eq!(
            xdg.headroom_accounts_dir(),
            PathBuf::from("/data/headroom/accounts/copilot")
        );
        let direct = config_with(&[("GH_CONFIG_DIR", "/gh"), ("XDG_CONFIG_HOME", "/cfg")]);
        assert_eq!(direct.gh_config_dir, PathBuf::from("/gh"));
        let relative = config_with(&[("XDG_CONFIG_HOME", "cfg"), ("GH_CONFIG_DIR", "")]);
        assert_eq!(relative.gh_config_dir, PathBuf::from("/home/u/.config/gh"));
    }
}
