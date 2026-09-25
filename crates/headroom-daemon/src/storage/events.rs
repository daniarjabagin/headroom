use headroom_core::cursor::LogCursors;
use headroom_core::event::{EventKey, UsageEvent};
use headroom_core::tokens::TokenCounts;
use headroom_core::units::MicroUsd;
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
     cache_read, cache_write_5m, cache_write_1h, output, reasoning, total, web_search, \
     reported_cost, project) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, \
     ?15, ?16) \
     ON CONFLICT(provider, usage_home, key) DO UPDATE SET at = excluded.at, \
     model = excluded.model, tier = excluded.tier, input = excluded.input, \
     cache_read = excluded.cache_read, cache_write_5m = excluded.cache_write_5m, \
     cache_write_1h = excluded.cache_write_1h, output = excluded.output, \
     reasoning = excluded.reasoning, total = excluded.total, web_search = excluded.web_search, \
     reported_cost = excluded.reported_cost, \
     project = COALESCE(excluded.project, usage_events.project) \
     WHERE excluded.total > usage_events.total";

const FILL_PROJECT: &str = "UPDATE usage_events SET project = ?4 \
     WHERE provider = ?1 AND usage_home = ?2 AND key = ?3 AND project IS NULL";

const SELECT_SINCE: &str = "SELECT key, at, model, tier, input, cache_read, cache_write_5m, \
     cache_write_1h, output, reasoning, web_search, reported_cost, project FROM usage_events \
     WHERE provider = ?1 AND usage_home = ?2 AND at >= ?3 ORDER BY at, key";

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Ingested {
    pub changed: usize,
    pub skipped: usize,
}

struct HomeKey<'a> {
    provider: String,
    home: &'a str,
}

struct EventRow<'a> {
    key: &'a str,
    at: i64,
    model: &'a str,
    tier: String,
    counts: [i64; 7],
    web_search: u32,
    reported_cost: Option<i64>,
    project: Option<&'a str>,
}

pub fn ingest(
    conn: &mut Connection,
    home: &UsageHome,
    events: &[UsageEvent],
    cursors: &LogCursors,
) -> Result<Ingested, StorageError> {
    let key = HomeKey {
        provider: enum_to_sql(&home.provider)?,
        home: path_to_sql(&home.home)?,
    };
    let tx = conn.transaction()?;
    let mut ingested = Ingested::default();
    for event in events {
        match encode(event) {
            Ok(row) => ingested.changed += store(&tx, &key, &row)?,
            Err(error) => {
                tracing::warn!(key = %event.key.0, %error, "skipped a usage event that cannot be stored");
                ingested.skipped += 1;
            }
        }
    }
    cursors::save(&tx, home, cursors)?;
    tx.commit()?;
    Ok(ingested)
}

fn encode(event: &UsageEvent) -> Result<EventRow<'_>, StorageError> {
    let tokens = &event.tokens;
    Ok(EventRow {
        key: &event.key.0,
        at: timestamp_to_sql(event.at)?,
        model: &event.model,
        tier: enum_to_sql(&event.tier)?,
        counts: [
            tokens_to_sql(tokens.input)?,
            tokens_to_sql(tokens.cache_read)?,
            tokens_to_sql(tokens.cache_write_5m)?,
            tokens_to_sql(tokens.cache_write_1h)?,
            tokens_to_sql(tokens.output)?,
            tokens_to_sql(tokens.reasoning)?,
            tokens_to_sql(tokens.total())?,
        ],
        web_search: event.web_search_requests,
        reported_cost: event.reported_cost.map(|cost| cost.0),
        project: event.project.as_deref(),
    })
}

fn store(
    tx: &Transaction<'_>,
    home: &HomeKey<'_>,
    row: &EventRow<'_>,
) -> Result<usize, StorageError> {
    let changed = upsert(tx, home, row)?;
    match row.project {
        Some(project) if changed == 0 => fill_project(tx, home, row.key, project),
        _ => Ok(changed),
    }
}

fn fill_project(
    tx: &Transaction<'_>,
    home: &HomeKey<'_>,
    key: &str,
    project: &str,
) -> Result<usize, StorageError> {
    Ok(tx
        .prepare_cached(FILL_PROJECT)?
        .execute(params![home.provider, home.home, key, project])?)
}

fn upsert(
    tx: &Transaction<'_>,
    home: &HomeKey<'_>,
    row: &EventRow<'_>,
) -> Result<usize, StorageError> {
    let [
        input,
        read,
        short_write,
        long_write,
        output,
        reasoning,
        total,
    ] = row.counts;
    Ok(tx.prepare_cached(UPSERT)?.execute(params![
        home.provider,
        home.home,
        row.key,
        row.at,
        row.model,
        row.tier,
        input,
        read,
        short_write,
        long_write,
        output,
        reasoning,
        total,
        row.web_search,
        row.reported_cost,
        row.project
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
    reported_cost: Option<i64>,
    project: Option<String>,
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
        reported_cost: row.get(11)?,
        project: row.get(12)?,
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
        reported_cost: raw.reported_cost.map(MicroUsd),
        project: raw.project,
    })
}

#[cfg(test)]
#[path = "events_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "events_project_tests.rs"]
mod project_tests;
