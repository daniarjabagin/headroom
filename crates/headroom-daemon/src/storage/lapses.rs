use headroom_core::account::AccountId;
use rusqlite::{Connection, params};

use crate::error::StorageError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lapse {
    pub account: AccountId,
    pub detail: String,
    pub notified: bool,
}

pub fn load_all(conn: &Connection) -> Result<Vec<Lapse>, StorageError> {
    let mut statement = conn.prepare(
        "SELECT account_id, detail, notified FROM subscription_lapses ORDER BY account_id",
    )?;
    let rows = statement
        .query_map([], |row| {
            Ok(Lapse {
                account: AccountId(row.get(0)?),
                detail: row.get(1)?,
                notified: row.get(2)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

pub fn record(conn: &Connection, account: &AccountId, detail: &str) -> Result<(), StorageError> {
    conn.execute(
        "INSERT INTO subscription_lapses (account_id, detail) VALUES (?1, ?2) \
         ON CONFLICT(account_id) DO UPDATE SET detail = excluded.detail",
        params![account.0, detail],
    )?;
    Ok(())
}

pub fn mark_notified(conn: &Connection, account: &AccountId) -> Result<(), StorageError> {
    conn.execute(
        "UPDATE subscription_lapses SET notified = 1 WHERE account_id = ?1",
        params![account.0],
    )?;
    Ok(())
}

pub fn clear(conn: &Connection, account: &AccountId) -> Result<(), StorageError> {
    conn.execute(
        "DELETE FROM subscription_lapses WHERE account_id = ?1",
        params![account.0],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::Storage;

    #[test]
    fn lapses_keep_the_notified_flag_until_cleared() {
        let storage = Storage::open_in_memory().unwrap();
        let id = AccountId("codex:work".into());
        let lapse = |detail: &str, notified: bool| Lapse {
            account: id.clone(),
            detail: detail.into(),
            notified,
        };
        storage.blocking(|conn| record(conn, &id, "first")).unwrap();
        assert_eq!(
            storage.blocking(|conn| load_all(conn)).unwrap(),
            [lapse("first", false)]
        );
        storage
            .blocking(|conn| {
                mark_notified(conn, &id)?;
                record(conn, &id, "second")
            })
            .unwrap();
        assert_eq!(
            storage.blocking(|conn| load_all(conn)).unwrap(),
            [lapse("second", true)]
        );
        storage.blocking(|conn| clear(conn, &id)).unwrap();
        assert!(storage.blocking(|conn| load_all(conn)).unwrap().is_empty());
    }
}
