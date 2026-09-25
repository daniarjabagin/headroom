use jiff::Timestamp;
use rusqlite::{Connection, params};
use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::error::StorageError;

pub fn load_all<T: DeserializeOwned>(conn: &Connection) -> Result<Vec<T>, StorageError> {
    let mut statement = conn.prepare("SELECT id, payload FROM held_alerts ORDER BY held_at, id")?;
    let rows = statement
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    let mut alerts = Vec::with_capacity(rows.len());
    for (id, payload) in rows {
        match serde_json::from_str(&payload) {
            Ok(alert) => alerts.push(alert),
            Err(error) => drop_unreadable(conn, &id, &error)?,
        }
    }
    Ok(alerts)
}

fn drop_unreadable(
    conn: &Connection,
    id: &str,
    error: &serde_json::Error,
) -> Result<(), StorageError> {
    tracing::warn!(id, %error, "dropping an unreadable held alert");
    conn.execute("DELETE FROM held_alerts WHERE id = ?1", params![id])?;
    Ok(())
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
