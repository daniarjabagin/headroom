use headroom_core::usage::{PriceBook, UsageSummary, aggregate};
use jiff::tz::TimeZone;
use jiff::{SignedDuration, Timestamp};
use rusqlite::Connection;

use crate::error::StorageError;
use crate::home::UsageHome;
use crate::storage::events;

pub const RETENTION: SignedDuration = SignedDuration::from_hours(35 * 24);

#[must_use]
pub fn retention_start(now: Timestamp) -> Timestamp {
    now.checked_sub(RETENTION).unwrap_or(Timestamp::MIN)
}

pub fn summarize(
    conn: &Connection,
    home: &UsageHome,
    prices: &dyn PriceBook,
    tz: &TimeZone,
    now: Timestamp,
) -> Result<UsageSummary, StorageError> {
    let events = events::load_since(conn, home, retention_start(now))?;
    Ok(aggregate(&events, prices, tz, now))
}

pub fn prune(conn: &Connection, now: Timestamp) -> Result<usize, StorageError> {
    events::prune_before(conn, retention_start(now))
}
