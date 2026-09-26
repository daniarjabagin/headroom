use headroom_core::account::AccountId;
use headroom_core::history::{SAMPLE_RETENTION, SampleStep, UsageSample, observed_at, sample_step};
use headroom_core::quota::{LimitsSnapshot, QuotaWindow};
use headroom_core::units::Percent;
use jiff::Timestamp;
use rusqlite::{Connection, OptionalExtension, params};

use super::codec::{timestamp_from_sql, timestamp_to_sql};
use crate::error::StorageError;
use crate::model::window_key;

pub type SampleRow = (AccountId, String, UsageSample);

pub fn record(
    conn: &Connection,
    id: &AccountId,
    snapshot: &LimitsSnapshot,
    now: Timestamp,
) -> Result<(), StorageError> {
    let at = observed_at(snapshot);
    for window in &snapshot.windows {
        record_window(conn, id, window, at)?;
    }
    prune(conn, now)
}

pub fn load_all(conn: &Connection) -> Result<Vec<SampleRow>, StorageError> {
    let mut statement = conn.prepare(
        "SELECT account_id, window_id, used_percent, at FROM quota_samples \
         ORDER BY account_id, window_id, at",
    )?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, f64>(2)?,
                row.get::<_, i64>(3)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    rows.into_iter()
        .map(|(id, window, used, at)| Ok((AccountId(id), window, sample(used, at)?)))
        .collect()
}

pub fn prune(conn: &Connection, now: Timestamp) -> Result<(), StorageError> {
    let cutoff = now.checked_sub(SAMPLE_RETENTION).unwrap_or(Timestamp::MIN);
    conn.execute(
        "DELETE FROM quota_samples WHERE at < ?1",
        params![timestamp_to_sql(cutoff)?],
    )?;
    Ok(())
}

fn record_window(
    conn: &Connection,
    id: &AccountId,
    window: &QuotaWindow,
    at: Timestamp,
) -> Result<(), StorageError> {
    let key = window_key(&window.id);
    let last = last_sample(conn, id, &key)?;
    match sample_step(last.as_ref(), window, at) {
        Some(SampleStep::Restart) => {
            clear(conn, id, &key)?;
            insert(conn, id, &key, window.used, at)
        }
        Some(SampleStep::Append) => insert(conn, id, &key, window.used, at),
        None => Ok(()),
    }
}

fn last_sample(
    conn: &Connection,
    id: &AccountId,
    key: &str,
) -> Result<Option<UsageSample>, StorageError> {
    let row = conn
        .query_row(
            "SELECT used_percent, at FROM quota_samples WHERE account_id = ?1 AND window_id = ?2 \
             ORDER BY at DESC LIMIT 1",
            params![id.0, key],
            |row| Ok((row.get::<_, f64>(0)?, row.get::<_, i64>(1)?)),
        )
        .optional()?;
    row.map(|(used, at)| sample(used, at)).transpose()
}

fn clear(conn: &Connection, id: &AccountId, key: &str) -> Result<(), StorageError> {
    conn.execute(
        "DELETE FROM quota_samples WHERE account_id = ?1 AND window_id = ?2",
        params![id.0, key],
    )?;
    Ok(())
}

fn insert(
    conn: &Connection,
    id: &AccountId,
    key: &str,
    used: Percent,
    at: Timestamp,
) -> Result<(), StorageError> {
    conn.execute(
        "INSERT OR REPLACE INTO quota_samples (account_id, window_id, used_percent, at) \
         VALUES (?1, ?2, ?3, ?4)",
        params![id.0, key, used.value(), timestamp_to_sql(at)?],
    )?;
    Ok(())
}

fn sample(used: f64, at: i64) -> Result<UsageSample, StorageError> {
    Ok(UsageSample {
        at: timestamp_from_sql(at)?,
        used: Percent::new(used),
    })
}

#[cfg(test)]
#[path = "samples_tests.rs"]
mod tests;
