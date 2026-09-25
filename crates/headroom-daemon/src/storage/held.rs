use jiff::Timestamp;
use rusqlite::{Connection, params};
use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::error::StorageError;

pub fn load_all<T: DeserializeOwned>(conn: &Connection) -> Result<Vec<T>, StorageError> {
    let mut statement = conn.prepare("SELECT payload FROM held_alerts ORDER BY held_at, id")?;
    let payloads = statement
        .query_map([], |row| row.get::<_, String>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    payloads
        .iter()
        .map(|payload| Ok(serde_json::from_str(payload)?))
        .collect()
}

pub fn save<T: Serialize>(
    conn: &Connection,
    id: &str,
    held_at: Timestamp,
    payload: &T,
) -> Result<(), StorageError> {
    conn.execute(
        "INSERT INTO held_alerts (id, held_at, payload) VALUES (?1, ?2, ?3) \
         ON CONFLICT(id) DO UPDATE SET held_at = excluded.held_at, payload = excluded.payload",
        params![id, held_at.as_second(), serde_json::to_string(payload)?],
    )?;
    Ok(())
}

pub fn delete(conn: &mut Connection, ids: &[String]) -> Result<(), StorageError> {
    let tx = conn.transaction()?;
    {
        let mut statement = tx.prepare("DELETE FROM held_alerts WHERE id = ?1")?;
        for id in ids {
            statement.execute(params![id])?;
        }
    }
    tx.commit()?;
    Ok(())
}

#[cfg(test)]
#[path = "held_tests.rs"]
mod tests;
