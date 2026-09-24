use std::ffi::OsString;
use std::fs::{self, DirBuilder, OpenOptions, Permissions};
use std::io;
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};

use crate::error::StorageError;

const PRIVATE_DIR: u32 = 0o700;
const PRIVATE_FILE: u32 = 0o600;
const SQLITE_SIDE_FILES: [&str; 2] = ["-wal", "-shm"];

pub fn prepare(path: &Path, app_dir: Option<&Path>) -> Result<(), StorageError> {
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        create_private_dir(parent)?;
        if app_dir == Some(parent) {
            restrict(parent, PRIVATE_DIR)?;
        }
    }
    create_private_file(path)?;
    for side in side_files(path) {
        if fs::symlink_metadata(&side).is_ok() {
            restrict(&side, PRIVATE_FILE)?;
        }
    }
    Ok(())
}

fn create_private_dir(dir: &Path) -> Result<(), StorageError> {
    DirBuilder::new()
        .recursive(true)
        .mode(PRIVATE_DIR)
        .create(dir)
        .map_err(|source| StorageError::CreateDir {
            path: dir.to_path_buf(),
            source,
        })
}

fn create_private_file(path: &Path) -> Result<(), StorageError> {
    OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .mode(PRIVATE_FILE)
        .open(path)
        .map_err(restrict_error(path))?;
    restrict(path, PRIVATE_FILE)
}

fn restrict(path: &Path, mode: u32) -> Result<(), StorageError> {
    fs::set_permissions(path, Permissions::from_mode(mode)).map_err(restrict_error(path))
}

fn restrict_error(path: &Path) -> impl FnOnce(io::Error) -> StorageError {
    let path = path.to_path_buf();
    move |source| StorageError::Restrict { path, source }
}

fn side_files(path: &Path) -> impl Iterator<Item = PathBuf> {
    SQLITE_SIDE_FILES.into_iter().map(move |suffix| {
        let mut name = OsString::from(path.as_os_str());
        name.push(suffix);
        PathBuf::from(name)
    })
}

#[cfg(test)]
#[path = "private_tests.rs"]
mod tests;
