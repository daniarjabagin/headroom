pub mod accounts;
pub mod alerts;
mod codec;
pub mod cursors;
pub mod dismissed;
pub mod events;
pub mod lapses;
mod migrations;
pub mod settings;
pub mod snapshots;

use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use rusqlite::Connection;

use crate::error::StorageError;

const BUSY_TIMEOUT: Duration = Duration::from_secs(5);
const PAGE_CACHE_KIB: i64 = 1024;

#[derive(Clone)]
pub struct Storage {
    conn: Arc<Mutex<Connection>>,
}

impl Storage {
    pub fn open(path: &Path) -> Result<Storage, StorageError> {
        create_parent(path)?;
        let conn = Connection::open(path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        Storage::prepare(conn)
    }

    #[cfg(test)]
    pub fn open_in_memory() -> Result<Storage, StorageError> {
        Storage::prepare(Connection::open_in_memory()?)
    }

    fn prepare(mut conn: Connection) -> Result<Storage, StorageError> {
        conn.busy_timeout(BUSY_TIMEOUT)?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.pragma_update(None, "cache_size", -PAGE_CACHE_KIB)?;
        migrations::migrate(&mut conn)?;
        Ok(Storage {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    pub fn blocking<R>(
        &self,
        work: impl FnOnce(&mut Connection) -> Result<R, StorageError>,
    ) -> Result<R, StorageError> {
        let mut conn = self.conn.lock().map_err(|_| StorageError::Poisoned)?;
        work(&mut conn)
    }

    pub async fn run<R: Send + 'static>(
        &self,
        work: impl FnOnce(&mut Connection) -> Result<R, StorageError> + Send + 'static,
    ) -> Result<R, StorageError> {
        let storage = self.clone();
        tokio::task::spawn_blocking(move || storage.blocking(work))
            .await
            .map_err(|error| StorageError::Task(error.to_string()))?
    }
}

fn create_parent(path: &Path) -> Result<(), StorageError> {
    let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) else {
        return Ok(());
    };
    std::fs::create_dir_all(parent).map_err(|source| StorageError::CreateDir {
        path: parent.to_path_buf(),
        source,
    })
}

#[cfg(test)]
mod tests;
