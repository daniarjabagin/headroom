use rusqlite::{Connection, OptionalExtension};

use crate::error::StorageError;
use crate::settings::Settings;

pub fn load(conn: &Connection) -> Result<Settings, StorageError> {
    let payload: Option<String> = conn
        .query_row("SELECT payload FROM settings WHERE id = 1", [], |row| {
            row.get(0)
        })
        .optional()?;
    match payload {
        Some(json) => Ok(serde_json::from_str(&json)?),
        None => Ok(Settings::default()),
    }
}

pub fn save(conn: &Connection, settings: &Settings) -> Result<(), StorageError> {
    conn.execute(
        "INSERT INTO settings (id, payload) VALUES (1, ?1) \
         ON CONFLICT(id) DO UPDATE SET payload = excluded.payload",
        [serde_json::to_string(settings)?],
    )?;
    Ok(())
}
