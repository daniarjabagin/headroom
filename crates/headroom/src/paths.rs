use std::path::PathBuf;

use anyhow::{Context, Result};
#[cfg(target_os = "linux")]
use headroom_daemon::BusTarget;
use headroom_providers::paths::HeadroomDirs;

pub struct Globals {
    #[cfg(target_os = "linux")]
    pub bus: BusTarget,
    pub db: Option<PathBuf>,
    pub socket: Option<PathBuf>,
}

impl Globals {
    pub fn db_path(&self) -> Result<PathBuf> {
        match &self.db {
            Some(path) => Ok(path.clone()),
            None => Ok(headroom_daemon::default_db_path()?),
        }
    }
}

pub fn pricing_cache_dir() -> Result<PathBuf> {
    let cache = dirs::cache_dir().context("no cache directory is available")?;
    Ok(cache.join("headroom").join("pricing"))
}

pub fn accounts_root() -> Result<PathBuf> {
    let dirs = HeadroomDirs::from_process().context("no home directory is available")?;
    Ok(dirs.accounts_root())
}

#[must_use]
pub fn socket_override(value: Option<std::ffi::OsString>) -> Option<PathBuf> {
    value.filter(|path| !path.is_empty()).map(PathBuf::from)
}
