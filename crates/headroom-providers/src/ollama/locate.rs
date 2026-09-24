use std::fs::File;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

const KEY_NAME: &str = "id_ed25519";
const SYSTEM_HOME: &str = "/usr/share/ollama/.ollama";
const MAX_KEY_BYTES: u64 = 64 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyPaths {
    pub user: PathBuf,
    pub system: PathBuf,
}

impl KeyPaths {
    #[must_use]
    pub fn from_home(home: &Path) -> KeyPaths {
        KeyPaths {
            user: home.join(".ollama").join(KEY_NAME),
            system: Path::new(SYSTEM_HOME).join(KEY_NAME),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum KeyFile {
    Found { path: PathBuf, pem: String },
    Unreadable { path: PathBuf },
    Missing,
}

impl KeyFile {
    pub(super) fn path(&self) -> Option<&Path> {
        match self {
            KeyFile::Found { path, .. } | KeyFile::Unreadable { path } => Some(path),
            KeyFile::Missing => None,
        }
    }
}

pub(super) fn locate(paths: &KeyPaths) -> KeyFile {
    match read(&paths.user) {
        KeyFile::Missing => read(&paths.system),
        found => found,
    }
}

fn read(path: &Path) -> KeyFile {
    match read_bounded(path) {
        Ok(pem) => KeyFile::Found {
            path: path.to_path_buf(),
            pem,
        },
        Err(error) if error.kind() == io::ErrorKind::NotFound => KeyFile::Missing,
        Err(error) => {
            tracing::debug!(path = %path.display(), kind = ?error.kind(), "cannot read ollama key");
            KeyFile::Unreadable {
                path: path.to_path_buf(),
            }
        }
    }
}

fn read_bounded(path: &Path) -> io::Result<String> {
    let mut text = String::new();
    File::open(path)?
        .take(MAX_KEY_BYTES)
        .read_to_string(&mut text)?;
    Ok(text)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;

    use super::*;

    fn paths(root: &Path) -> KeyPaths {
        KeyPaths {
            user: root.join("home/.ollama/id_ed25519"),
            system: root.join("usr/share/ollama/.ollama/id_ed25519"),
        }
    }

    fn write(path: &Path, text: &str) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }

    #[test]
    fn the_user_key_wins_over_the_system_key() {
        let root = tempfile::tempdir().unwrap();
        let paths = paths(root.path());
        write(&paths.user, "user");
        write(&paths.system, "system");
        assert_eq!(
            locate(&paths),
            KeyFile::Found {
                path: paths.user.clone(),
                pem: "user".into()
            }
        );
    }

    #[test]
    fn a_readable_system_key_is_used_when_the_user_has_none() {
        let root = tempfile::tempdir().unwrap();
        let paths = paths(root.path());
        write(&paths.system, "system");
        assert_eq!(locate(&paths).path(), Some(paths.system.as_path()));
    }

    #[test]
    fn nothing_on_disk_is_missing() {
        let root = tempfile::tempdir().unwrap();
        assert_eq!(locate(&paths(root.path())), KeyFile::Missing);
    }

    #[test]
    fn a_key_that_cannot_be_read_is_reported_as_unreadable() {
        let root = tempfile::tempdir().unwrap();
        let paths = paths(root.path());
        fs::create_dir_all(&paths.system).unwrap();
        assert_eq!(
            locate(&paths),
            KeyFile::Unreadable {
                path: paths.system.clone()
            }
        );
    }

    #[test]
    fn a_locked_system_home_is_reported_as_unreadable() {
        let root = tempfile::tempdir().unwrap();
        let paths = paths(root.path());
        write(&paths.system, "system");
        let home = paths.system.parent().unwrap();
        fs::set_permissions(home, fs::Permissions::from_mode(0o000)).unwrap();
        let located = locate(&paths);
        fs::set_permissions(home, fs::Permissions::from_mode(0o700)).unwrap();
        if running_as_root() {
            return;
        }
        assert_eq!(
            located,
            KeyFile::Unreadable {
                path: paths.system.clone()
            }
        );
    }

    fn running_as_root() -> bool {
        rustix::process::geteuid().is_root()
    }

    #[test]
    fn system_paths_follow_the_official_installer() {
        let paths = KeyPaths::from_home(Path::new("/home/someone"));
        assert_eq!(paths.user, Path::new("/home/someone/.ollama/id_ed25519"));
        assert_eq!(
            paths.system,
            Path::new("/usr/share/ollama/.ollama/id_ed25519")
        );
    }
}
