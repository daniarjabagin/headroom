use headroom_core::account::AccountId;
use headroom_core::quota::LimitsSnapshot;
use rusqlite::{Connection, params};

use super::codec::timestamp_to_sql;
use crate::error::StorageError;

pub fn save(
    conn: &Connection,
    id: &AccountId,
    snapshot: &LimitsSnapshot,
) -> Result<(), StorageError> {
    conn.execute(
        "INSERT INTO limits_snapshots (account_id, payload, fetched_at) VALUES (?1, ?2, ?3) \
         ON CONFLICT(account_id) DO UPDATE SET payload = excluded.payload, \
         fetched_at = excluded.fetched_at",
        params![
            id.0,
            serde_json::to_string(snapshot)?,
            timestamp_to_sql(snapshot.fetched_at)?
        ],
    )?;
    Ok(())
}

pub fn delete(conn: &Connection, id: &AccountId) -> Result<(), StorageError> {
    conn.execute(
        "DELETE FROM limits_snapshots WHERE account_id = ?1",
        params![id.0],
    )?;
    Ok(())
}

pub fn load_all(conn: &Connection) -> Result<Vec<(AccountId, LimitsSnapshot)>, StorageError> {
    let mut statement =
        conn.prepare("SELECT account_id, payload FROM limits_snapshots ORDER BY account_id")?;
    let rows = statement
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    rows.into_iter()
        .map(|(id, payload)| Ok((AccountId(id), serde_json::from_str(&payload)?)))
        .collect()
}
