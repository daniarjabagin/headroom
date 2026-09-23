use std::path::PathBuf;

use anyhow::{Context, Result};
use headroom_daemon::BusTarget;

pub struct Globals {
    pub bus: BusTarget,
    pub db: Option<PathBuf>,
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
    let cache = dirs::cache_dir().context("no XDG cache directory is available")?;
    Ok(cache.join("headroom").join("pricing"))
}

pub fn accounts_root() -> Result<PathBuf> {
    let data = dirs::data_dir().context("no XDG data directory is available")?;
    Ok(data.join("headroom").join("accounts"))
}
