use std::ffi::OsString;
use std::path::{Path, PathBuf};

const LINUX_DATA_HOME: &str = ".local/share";
const LINUX_CONFIG_HOME: &str = ".config";
const MACOS_APP_SUPPORT: &str = "Library/Application Support";
const LINUX_NAME: &str = "headroom";
const MACOS_NAME: &str = "Headroom";
const ACCOUNTS_DIR: &str = "accounts";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Os {
    Linux,
    MacOs,
}

impl Os {
    #[must_use]
    pub const fn current() -> Os {
        if cfg!(target_os = "macos") {
            Os::MacOs
        } else {
            Os::Linux
        }
    }
}

/// Where Headroom keeps what it owns: account homes and the secrets fallback.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeadroomDirs {
    pub data: PathBuf,
}

impl HeadroomDirs {
    #[must_use]
    pub fn for_os(os: Os, home: &Path, var: impl Fn(&str) -> Option<OsString>) -> HeadroomDirs {
        let data = match os {
            Os::Linux => absolute_var(&var, "XDG_DATA_HOME")
                .unwrap_or_else(|| home.join(LINUX_DATA_HOME))
                .join(LINUX_NAME),
            Os::MacOs => home.join(MACOS_APP_SUPPORT).join(MACOS_NAME),
        };
        HeadroomDirs { data }
    }

    #[must_use]
    pub fn from_vars(home: &Path, var: impl Fn(&str) -> Option<OsString>) -> HeadroomDirs {
        HeadroomDirs::for_os(Os::current(), home, var)
    }

    #[must_use]
    pub fn for_home(home: &Path) -> HeadroomDirs {
        HeadroomDirs::from_vars(home, |_| None)
    }

    #[must_use]
    pub fn from_process() -> Option<HeadroomDirs> {
        let home = dirs::home_dir()?;
        Some(HeadroomDirs::from_vars(&home, |name| {
            std::env::var_os(name)
        }))
    }

    #[must_use]
    pub fn accounts_root(&self) -> PathBuf {
        self.data.join(ACCOUNTS_DIR)
    }

    #[must_use]
    pub fn accounts(&self, provider: &str) -> PathBuf {
        self.accounts_root().join(provider)
    }

    #[must_use]
    pub fn secrets(&self) -> PathBuf {
        self.data.join("secrets")
    }
}

/// Where desktop apps keep their settings: `$XDG_CONFIG_HOME` on Linux, Application Support on macOS.
#[must_use]
pub fn app_config_dir(os: Os, home: &Path, var: impl Fn(&str) -> Option<OsString>) -> PathBuf {
    match os {
        Os::Linux => xdg_config_home(home, var),
        Os::MacOs => home.join(MACOS_APP_SUPPORT),
    }
}

#[must_use]
pub fn xdg_config_home(home: &Path, var: impl Fn(&str) -> Option<OsString>) -> PathBuf {
    absolute_var(&var, "XDG_CONFIG_HOME").unwrap_or_else(|| home.join(LINUX_CONFIG_HOME))
}

fn absolute_var(var: &impl Fn(&str) -> Option<OsString>, name: &str) -> Option<PathBuf> {
    var(name)
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vars(pairs: &[(&'static str, &'static str)]) -> impl Fn(&str) -> Option<OsString> {
        let pairs = pairs.to_vec();
        move |name| {
            pairs
                .iter()
                .find(|(key, _)| *key == name)
                .map(|(_, value)| OsString::from(value))
        }
    }

    #[test]
    fn linux_data_follows_xdg_data_home() {
        let home = Path::new("/home/u");
        let default = HeadroomDirs::for_os(Os::Linux, home, vars(&[]));
        assert_eq!(default.data, Path::new("/home/u/.local/share/headroom"));
        let custom = HeadroomDirs::for_os(Os::Linux, home, vars(&[("XDG_DATA_HOME", "/data")]));
        assert_eq!(custom.data, Path::new("/data/headroom"));
        let relative = HeadroomDirs::for_os(Os::Linux, home, vars(&[("XDG_DATA_HOME", "rel")]));
        assert_eq!(relative, default);
    }

    #[test]
    fn macos_data_lives_in_application_support() {
        let dirs = HeadroomDirs::for_os(
            Os::MacOs,
            Path::new("/Users/u"),
            vars(&[("XDG_DATA_HOME", "/data")]),
        );
        assert_eq!(
            dirs.data,
            Path::new("/Users/u/Library/Application Support/Headroom")
        );
        assert_eq!(
            dirs.accounts("claude"),
            Path::new("/Users/u/Library/Application Support/Headroom/accounts/claude")
        );
        assert_eq!(
            dirs.secrets(),
            Path::new("/Users/u/Library/Application Support/Headroom/secrets")
        );
    }

    #[test]
    fn app_config_dir_differs_per_os() {
        let home = Path::new("/h");
        let custom = vars(&[("XDG_CONFIG_HOME", "/cfg")]);
        assert_eq!(app_config_dir(Os::Linux, home, &custom), Path::new("/cfg"));
        assert_eq!(
            app_config_dir(Os::Linux, home, vars(&[])),
            Path::new("/h/.config")
        );
        assert_eq!(
            app_config_dir(Os::MacOs, home, &custom),
            Path::new("/h/Library/Application Support")
        );
    }

    #[test]
    fn the_current_os_matches_the_build_target() {
        let expected = if cfg!(target_os = "macos") {
            Os::MacOs
        } else {
            Os::Linux
        };
        assert_eq!(Os::current(), expected);
    }
}
