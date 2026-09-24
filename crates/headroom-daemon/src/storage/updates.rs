use jiff::Timestamp;
use rusqlite::{Connection, OptionalExtension, Row, params};

use super::codec::{timestamp_from_sql, timestamp_to_sql};
use crate::error::StorageError;
use crate::update::release::Release;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckRecord {
    pub checked_at: Timestamp,
    pub etag: Option<String>,
    pub latest: Option<Release>,
}

type RawRecord = (
    i64,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<i64>,
);

pub fn load(conn: &Connection) -> Result<Option<CheckRecord>, StorageError> {
    let raw = conn
        .query_row(
            "SELECT checked_at, etag, version, url, published_at FROM update_check WHERE id = 1",
            [],
            raw_record,
        )
        .optional()?;
    raw.map(decode).transpose()
}

pub fn save(conn: &Connection, record: &CheckRecord) -> Result<(), StorageError> {
    let latest = record.latest.as_ref();
    let published_at = latest
        .map(|release| timestamp_to_sql(release.published_at))
        .transpose()?;
    conn.execute(
        "INSERT INTO update_check (id, checked_at, etag, version, url, published_at) \
         VALUES (1, ?1, ?2, ?3, ?4, ?5) \
         ON CONFLICT(id) DO UPDATE SET checked_at = excluded.checked_at, etag = excluded.etag, \
         version = excluded.version, url = excluded.url, published_at = excluded.published_at",
        params![
            timestamp_to_sql(record.checked_at)?,
            record.etag,
            latest.map(|release| release.version.to_string()),
            latest.map(|release| release.url.clone()),
            published_at,
        ],
    )?;
    Ok(())
}

fn raw_record(row: &Row<'_>) -> rusqlite::Result<RawRecord> {
    Ok((
        row.get(0)?,
        row.get(1)?,
        row.get(2)?,
        row.get(3)?,
        row.get(4)?,
    ))
}

fn decode(
    (checked_at, etag, version, url, published_at): RawRecord,
) -> Result<CheckRecord, StorageError> {
    let latest = match (version, url, published_at) {
        (Some(version), Some(url), Some(published_at)) => Some(Release {
            version: version.parse().map_err(|_| StorageError::UnknownValue {
                field: "update version",
                value: version,
            })?,
            url,
            published_at: timestamp_from_sql(published_at)?,
        }),
        _ => None,
    };
    Ok(CheckRecord {
        checked_at: timestamp_from_sql(checked_at)?,
        etag,
        latest,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::Storage;

    fn release() -> Release {
        Release {
            version: "0.5.0".parse().unwrap(),
            url: "https://github.com/daniarjabagin/headroom/releases/tag/v0.5.0".into(),
            published_at: "2026-10-01T09:20:02Z".parse().unwrap(),
        }
    }

    #[test]
    fn the_last_check_round_trips_and_is_replaced() {
        let storage = Storage::open_in_memory().unwrap();
        assert_eq!(storage.blocking(|conn| load(conn)).unwrap(), None);
        let first = CheckRecord {
            checked_at: "2026-10-02T08:00:00Z".parse().unwrap(),
            etag: Some("W/\"abc\"".into()),
            latest: Some(release()),
        };
        storage.blocking(|conn| save(conn, &first)).unwrap();
        assert_eq!(storage.blocking(|conn| load(conn)).unwrap(), Some(first));
        let second = CheckRecord {
            checked_at: "2026-10-03T08:00:00Z".parse().unwrap(),
            etag: None,
            latest: None,
        };
        storage.blocking(|conn| save(conn, &second)).unwrap();
        assert_eq!(storage.blocking(|conn| load(conn)).unwrap(), Some(second));
    }
}
