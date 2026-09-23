use std::path::{Path, PathBuf};

use headroom_core::account::{AccountRef, ProviderKind};

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct UsageHome {
    pub provider: ProviderKind,
    pub home: PathBuf,
}

impl UsageHome {
    #[must_use]
    pub fn of(account: &AccountRef) -> UsageHome {
        UsageHome {
            provider: account.provider,
            home: account.home.clone(),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HomeDisplay {
    user_home: Option<PathBuf>,
}

impl HomeDisplay {
    #[must_use]
    pub fn new(user_home: Option<PathBuf>) -> HomeDisplay {
        HomeDisplay { user_home }
    }

    #[must_use]
    pub fn show(&self, path: &Path) -> String {
        let relative = self
            .user_home
            .as_deref()
            .and_then(|home| path.strip_prefix(home).ok());
        match relative {
            Some(rest) if rest.as_os_str().is_empty() => "~".to_owned(),
            Some(rest) => format!("~/{}", rest.display()),
            None => path.display().to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contracts_paths_below_the_user_home() {
        let display = HomeDisplay::new(Some(PathBuf::from("/home/ada")));
        assert_eq!(display.show(Path::new("/home/ada/.codex")), "~/.codex");
        assert_eq!(display.show(Path::new("/home/ada")), "~");
        assert_eq!(
            display.show(Path::new("/home/adam/.codex")),
            "/home/adam/.codex"
        );
        assert_eq!(display.show(Path::new("/srv/claude")), "/srv/claude");
    }

    #[test]
    fn without_user_home_paths_are_shown_as_is() {
        let display = HomeDisplay::default();
        assert_eq!(
            display.show(Path::new("/home/ada/.codex")),
            "/home/ada/.codex"
        );
    }
}
