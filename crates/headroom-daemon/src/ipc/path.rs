use std::fs::{self, DirBuilder};
use std::io::ErrorKind;
use std::os::unix::fs::{DirBuilderExt, MetadataExt};
use std::path::{Path, PathBuf};

use crate::config::app_dir;
use crate::error::{DaemonError, PrivacyIssue, SocketError};

pub const SOCKET_ENV: &str = "HEADROOM_SOCKET";
pub const MAX_SOCKET_PATH: usize = 103;
const SOCKET_NAME: &str = "daemon.sock";
const PRIVATE_MODE: u32 = 0o700;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SocketPath {
    Preferred(PathBuf),
    Private { dir: PathBuf, socket: PathBuf },
}

/// The default socket; a fallback in the temp dir lives in a verified per-user 0700 directory.
pub fn default_socket_path() -> Result<PathBuf, DaemonError> {
    let preferred = preferred_dir()?.join(SOCKET_NAME);
    let uid = rustix::process::getuid().as_raw();
    match fit_socket_path(preferred, &std::env::temp_dir(), uid) {
        SocketPath::Preferred(socket) => Ok(socket),
        SocketPath::Private { dir, socket } => {
            ensure_private_dir(&dir, uid)?;
            Ok(socket)
        }
    }
}

#[must_use]
pub fn fit_socket_path(preferred: PathBuf, temp_dir: &Path, uid: u32) -> SocketPath {
    if preferred.as_os_str().len() <= MAX_SOCKET_PATH {
        return SocketPath::Preferred(preferred);
    }
    let dir = temp_dir.join(format!("headroom-{uid}"));
    let socket = dir.join(SOCKET_NAME);
    SocketPath::Private { dir, socket }
}

pub fn ensure_private_dir(dir: &Path, uid: u32) -> Result<(), SocketError> {
    match DirBuilder::new().mode(PRIVATE_MODE).create(dir) {
        Ok(()) => {}
        Err(error) if error.kind() == ErrorKind::AlreadyExists => {}
        Err(source) => return Err(io_error("create", dir, source)),
    }
    let metadata = fs::symlink_metadata(dir).map_err(|source| io_error("inspect", dir, source))?;
    match privacy_issue(metadata.is_dir(), metadata.uid(), metadata.mode(), uid) {
        None => Ok(()),
        Some(issue) => Err(SocketError::NotPrivate {
            path: dir.to_path_buf(),
            issue,
        }),
    }
}

#[must_use]
pub fn privacy_issue(is_dir: bool, owner: u32, mode: u32, uid: u32) -> Option<PrivacyIssue> {
    if !is_dir {
        Some(PrivacyIssue::NotADirectory)
    } else if owner != uid {
        Some(PrivacyIssue::Owner { owner, uid })
    } else if mode & 0o777 != PRIVATE_MODE {
        Some(PrivacyIssue::Mode(mode & 0o777))
    } else {
        None
    }
}

fn io_error(action: &'static str, path: &Path, source: std::io::Error) -> SocketError {
    SocketError::Io {
        action,
        path: path.to_path_buf(),
        source,
    }
}

#[cfg(target_os = "linux")]
fn preferred_dir() -> Result<PathBuf, DaemonError> {
    match dirs::runtime_dir() {
        Some(runtime) => Ok(runtime.join("headroom")),
        None => app_dir(),
    }
}

#[cfg(not(target_os = "linux"))]
fn preferred_dir() -> Result<PathBuf, DaemonError> {
    app_dir()
}

#[cfg(test)]
#[path = "path_tests.rs"]
mod tests;
