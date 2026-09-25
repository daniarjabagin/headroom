use jiff::Timestamp;
use rusqlite::{Connection, Row, params};

use super::codec::{timestamp_from_sql, timestamp_to_sql};
use crate::error::StorageError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CachedPage {
    pub url: String,
    pub fetched_at: Timestamp,
    pub etag: Option<String>,
    pub body: String,
}

type RawPage = (String, i64, Option<String>, String);

pub fn load_all(conn: &Connection) -> Result<Vec<CachedPage>, StorageError> {
    let mut statement =
        conn.prepare("SELECT url, fetched_at, etag, body FROM status_cache ORDER BY url")?;
    let raw = statement
        .query_map([], raw_page)?
        .collect::<Result<Vec<_>, _>>()?;
    raw.into_iter().map(decode).collect()
}

pub fn save(conn: &Connection, page: &CachedPage) -> Result<(), StorageError> {
    conn.execute(
        "INSERT INTO status_cache (url, fetched_at, etag, body) VALUES (?1, ?2, ?3, ?4) \
         ON CONFLICT(url) DO UPDATE SET fetched_at = excluded.fetched_at, \
         etag = excluded.etag, body = excluded.body",
        params![
            page.url,
            timestamp_to_sql(page.fetched_at)?,
            page.etag,
            page.body
        ],
    )?;
    Ok(())
}

fn raw_page(row: &Row<'_>) -> rusqlite::Result<RawPage> {
    Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
}

fn decode((url, fetched_at, etag, body): RawPage) -> Result<CachedPage, StorageError> {
    Ok(CachedPage {
        url,
        fetched_at: timestamp_from_sql(fetched_at)?,
        etag,
        body,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::Storage;

    fn page(url: &str, at: &str, etag: Option<&str>, body: &str) -> CachedPage {
        CachedPage {
            url: url.into(),
            fetched_at: at.parse().unwrap(),
            etag: etag.map(str::to_owned),
            body: body.into(),
        }
    }

    #[test]
    fn pages_round_trip_and_are_replaced_by_url() {
        let storage = Storage::open_in_memory().unwrap();
        assert_eq!(storage.blocking(|conn| load_all(conn)).unwrap(), []);
        let claude = page(
            "https://status.claude.com/api/v2/summary.json",
            "2026-09-23T10:00:00Z",
            Some("W/\"e1\""),
            "{}",
        );
        let openai = page(
            "https://status.openai.com/api/v2/components.json",
            "2026-09-23T10:01:00Z",
            None,
            "{\"components\":[]}",
        );
        storage.blocking(|conn| save(conn, &claude)).unwrap();
        storage.blocking(|conn| save(conn, &openai)).unwrap();
        let newer = page(&claude.url, "2026-09-23T10:05:00Z", None, "{\"a\":1}");
        storage.blocking(|conn| save(conn, &newer)).unwrap();
        assert_eq!(
            storage.blocking(|conn| load_all(conn)).unwrap(),
            [newer, openai]
        );
    }
}
