use headroom_core::calibration::SpendPoint;
use headroom_core::event::UsageEvent;
use headroom_core::usage::{PriceBook, event_cost};
use jiff::{SignedDuration, Timestamp};
use rusqlite::Connection;

use crate::error::StorageError;
use crate::home::UsageHome;
use crate::storage::events;

pub const SPEND_LOOKBACK: SignedDuration = SignedDuration::from_hours(26);

pub fn recent_spend(
    conn: &Connection,
    home: &UsageHome,
    prices: &dyn PriceBook,
    now: Timestamp,
) -> Result<Vec<SpendPoint>, StorageError> {
    let since = now.checked_sub(SPEND_LOOKBACK).unwrap_or(Timestamp::MIN);
    Ok(spend_points(
        &events::load_since(conn, home, since)?,
        prices,
    ))
}

#[must_use]
pub fn spend_points(events: &[UsageEvent], prices: &dyn PriceBook) -> Vec<SpendPoint> {
    let mut points: Vec<SpendPoint> = events
        .iter()
        .filter_map(|event| {
            let cost = event_cost(event, prices)?;
            Some(SpendPoint { at: event.at, cost })
        })
        .collect();
    points.sort_unstable();
    points
}

#[must_use]
pub fn merge_spend<'a>(homes: impl IntoIterator<Item = &'a [SpendPoint]>) -> Vec<SpendPoint> {
    let mut points: Vec<SpendPoint> = homes.into_iter().flatten().copied().collect();
    points.sort_unstable();
    points
}

#[cfg(test)]
#[path = "weighted_tests.rs"]
mod tests;
