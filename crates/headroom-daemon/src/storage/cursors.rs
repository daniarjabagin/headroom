use headroom_core::cursor::LogCursors;
use rusqlite::{Connection, OptionalExtension, params};

use super::codec::{enum_to_sql, path_to_sql};
use crate::error::StorageError;
use crate::home::UsageHome;

pub fn load(conn: &Connection, home: &UsageHome) -> Result<LogCursors, StorageError> {
    let payload: Option<String> = conn
        .query_row(
            "SELECT payload FROM log_cursors WHERE provider = ?1 AND usage_home = ?2",
            params![enum_to_sql(&home.provider)?, path_to_sql(&home.home)?],
            |row| row.get(0),
        )
        .optional()?;
    match payload {
        Some(json) => Ok(serde_json::from_str(&json)?),
        None => Ok(LogCursors::default()),
    }
}

pub fn save(conn: &Connection, home: &UsageHome, cursors: &LogCursors) -> Result<(), StorageError> {
    conn.execute(
        "INSERT INTO log_cursors (provider, usage_home, payload) VALUES (?1, ?2, ?3) \
         ON CONFLICT(provider, usage_home) DO UPDATE SET payload = excluded.payload",
        params![
            enum_to_sql(&home.provider)?,
            path_to_sql(&home.home)?,
            serde_json::to_string(cursors)?
        ],
    )?;
    Ok(())
}
