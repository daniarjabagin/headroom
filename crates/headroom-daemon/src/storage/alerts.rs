use headroom_core::account::AccountId;
use rusqlite::{Connection, params};
use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::error::StorageError;

pub fn load_all<T: DeserializeOwned>(
    conn: &Connection,
) -> Result<Vec<(AccountId, String, T)>, StorageError> {
    let mut statement = conn.prepare(
        "SELECT account_id, window_id, payload FROM notification_state \
         ORDER BY account_id, window_id",
    )?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    rows.into_iter()
        .map(|(id, window, payload)| Ok((AccountId(id), window, serde_json::from_str(&payload)?)))
        .collect()
}

pub fn save<T: Serialize>(
    conn: &Connection,
    account: &AccountId,
    window: &str,
    state: &T,
) -> Result<(), StorageError> {
    conn.execute(
        "INSERT INTO notification_state (account_id, window_id, payload) VALUES (?1, ?2, ?3) \
         ON CONFLICT(account_id, window_id) DO UPDATE SET payload = excluded.payload",
        params![account.0, window, serde_json::to_string(state)?],
    )?;
    Ok(())
}
