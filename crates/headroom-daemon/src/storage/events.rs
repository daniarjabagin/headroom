use headroom_core::cursor::LogCursors;
use headroom_core::event::{EventKey, UsageEvent};
use headroom_core::tokens::TokenCounts;
use jiff::Timestamp;
use rusqlite::{Connection, Row, Transaction, params};

use super::codec::{
    enum_from_sql, enum_to_sql, path_to_sql, timestamp_from_sql, timestamp_to_sql, tokens_from_sql,
    tokens_to_sql,
};
use super::cursors;
use crate::error::StorageError;
use crate::home::UsageHome;

const UPSERT: &str = "INSERT INTO usage_events (provider, usage_home, key, at, model, tier, input, \
     cache_read, cache_write_5m, cache_write_1h, output, reasoning, total, web_search) \
     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14) \
     ON CONFLICT(provider, usage_home, key) DO UPDATE SET at = excluded.at, \
     model = excluded.model, tier = excluded.tier, input = excluded.input, \
     cache_read = excluded.cache_read, cache_write_5m = excluded.cache_write_5m, \
     cache_write_1h = excluded.cache_write_1h, output = excluded.output, \
     reasoning = excluded.reasoning, total = excluded.total, web_search = excluded.web_search \
     WHERE excluded.total > usage_events.total";

const SELECT_SINCE: &str = "SELECT key, at, model, tier, input, cache_read, cache_write_5m, \
     cache_write_1h, output, reasoning, web_search FROM usage_events \
     WHERE provider = ?1 AND usage_home = ?2 AND at >= ?3 ORDER BY at, key";

pub fn ingest(
    conn: &mut Connection,
    home: &UsageHome,
    events: &[UsageEvent],
    cursors: &LogCursors,
) -> Result<usize, StorageError> {
    let tx = conn.transaction()?;
    let mut changed = 0;
    for event in events {
        changed += upsert(&tx, home, event)?;
    }
    cursors::save(&tx, home, cursors)?;
    tx.commit()?;
    Ok(changed)
}

fn upsert(
    tx: &Transaction<'_>,
    home: &UsageHome,
    event: &UsageEvent,
) -> Result<usize, StorageError> {
    let tokens = &event.tokens;
    Ok(tx.prepare_cached(UPSERT)?.execute(params![
        enum_to_sql(&home.provider)?,
        path_to_sql(&home.home)?,
        event.key.0,
        timestamp_to_sql(event.at)?,
        event.model,
        enum_to_sql(&event.tier)?,
        tokens_to_sql(tokens.input)?,
        tokens_to_sql(tokens.cache_read)?,
        tokens_to_sql(tokens.cache_write_5m)?,
        tokens_to_sql(tokens.cache_write_1h)?,
        tokens_to_sql(tokens.output)?,
        tokens_to_sql(tokens.reasoning)?,
        tokens_to_sql(tokens.total())?,
        event.web_search_requests
    ])?)
}

pub fn load_since(
    conn: &Connection,
    home: &UsageHome,
    since: Timestamp,
) -> Result<Vec<UsageEvent>, StorageError> {
    let mut statement = conn.prepare_cached(SELECT_SINCE)?;
    let mut rows = statement.query(params![
        enum_to_sql(&home.provider)?,
        path_to_sql(&home.home)?,
        timestamp_to_sql(since)?
    ])?;
    let mut events = Vec::new();
    while let Some(row) = rows.next()? {
        events.push(decode(read_raw(row)?)?);
    }
    Ok(events)
}

pub fn prune_before(conn: &Connection, cutoff: Timestamp) -> Result<usize, StorageError> {
    Ok(conn.execute(
        "DELETE FROM usage_events WHERE at < ?1",
        [timestamp_to_sql(cutoff)?],
    )?)
}

struct RawEvent {
    key: String,
    at: i64,
    model: String,
    tier: String,
    counts: [i64; 6],
    web_search: u32,
}

fn read_raw(row: &Row<'_>) -> rusqlite::Result<RawEvent> {
    Ok(RawEvent {
        key: row.get(0)?,
        at: row.get(1)?,
        model: row.get(2)?,
        tier: row.get(3)?,
        counts: [
            row.get(4)?,
            row.get(5)?,
            row.get(6)?,
            row.get(7)?,
            row.get(8)?,
            row.get(9)?,
        ],
        web_search: row.get(10)?,
    })
}

fn decode(raw: RawEvent) -> Result<UsageEvent, StorageError> {
    let [input, read, short_write, long_write, output, reasoning] = raw.counts;
    Ok(UsageEvent {
        key: EventKey(raw.key),
        at: timestamp_from_sql(raw.at)?,
        model: raw.model,
        tier: enum_from_sql("tier", raw.tier)?,
        tokens: TokenCounts {
            input: tokens_from_sql(input)?,
            cache_read: tokens_from_sql(read)?,
            cache_write_5m: tokens_from_sql(short_write)?,
            cache_write_1h: tokens_from_sql(long_write)?,
            output: tokens_from_sql(output)?,
            reasoning: tokens_from_sql(reasoning)?,
        },
        web_search_requests: raw.web_search,
    })
}
