use std::fs::{self, DirBuilder, File, OpenOptions, Permissions};
use std::io::ErrorKind;
use std::os::unix::fs::{DirBuilderExt, FileTypeExt, MetadataExt, OpenOptionsExt, PermissionsExt};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};

use rustix::fs::{FlockOperation, flock};
use rustix::io::Errno;
use tokio::net::UnixListener;

use crate::error::SocketError;

const DIR_MODE: u32 = 0o700;
const SOCKET_MODE: u32 = 0o600;
const LOCK_MODE: u32 = 0o600;

pub struct SocketFile {
    path: PathBuf,
    identity: (u64, u64),
    _lock: File,
}

pub fn bind(path: &Path) -> Result<(UnixListener, SocketFile), SocketError> {
    prepare_parent(path)?;
    let lock = lock_sibling(path)?;
    clear_stale(path)?;
    let listener = UnixListener::bind(path).map_err(|source| match source.kind() {
        ErrorKind::AddrInUse => SocketError::AlreadyListening(path.to_path_buf()),
        _ => io_error("bind", path, source),
    })?;
    let file = SocketFile {
        path: path.to_path_buf(),
        identity: identity(path).map_err(|source| io_error("inspect", path, source))?,
        _lock: lock,
    };
    fs::set_permissions(path, Permissions::from_mode(SOCKET_MODE))
        .map_err(|source| io_error("restrict", path, source))?;
    Ok((listener, file))
}

impl SocketFile {
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for SocketFile {
    fn drop(&mut self) {
        if identity(&self.path).is_ok_and(|current| current == self.identity) {
            fs::remove_file(&self.path).ok();
        }
    }
}

fn prepare_parent(path: &Path) -> Result<(), SocketError> {
    let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) else {
        return Ok(());
    };
    if parent.exists() {
        return Ok(());
    }
    DirBuilder::new()
        .recursive(true)
        .mode(DIR_MODE)
        .create(parent)
        .map_err(|source| io_error("create", parent, source))
}

fn lock_sibling(socket: &Path) -> Result<File, SocketError> {
    let path = socket.with_extension("lock");
    let file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .mode(LOCK_MODE)
        .open(&path)
        .map_err(|source| io_error("open", &path, source))?;
    match flock(&file, FlockOperation::NonBlockingLockExclusive) {
        Ok(()) => Ok(file),
        Err(Errno::WOULDBLOCK) => Err(SocketError::AlreadyListening(socket.to_path_buf())),
        Err(errno) => Err(io_error("lock", &path, errno.into())),
    }
}

fn clear_stale(path: &Path) -> Result<(), SocketError> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(()),
        Err(source) => return Err(io_error("inspect", path, source)),
    };
    if !metadata.file_type().is_socket() {
        return Err(SocketError::NotASocket(path.to_path_buf()));
    }
    if UnixStream::connect(path).is_ok() {
        return Err(SocketError::AlreadyListening(path.to_path_buf()));
    }
    tracing::info!(path = %path.display(), "removing a stale socket");
    fs::remove_file(path).map_err(|source| io_error("remove", path, source))
}

fn identity(path: &Path) -> std::io::Result<(u64, u64)> {
    let metadata = fs::symlink_metadata(path)?;
    Ok((metadata.dev(), metadata.ino()))
}

fn io_error(action: &'static str, path: &Path, source: std::io::Error) -> SocketError {
    SocketError::Io {
        action,
        path: path.to_path_buf(),
        source,
    }
}
