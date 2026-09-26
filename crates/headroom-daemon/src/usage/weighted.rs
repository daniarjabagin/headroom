use headroom_core::calibration::SpendPoint;
use headroom_core::event::UsageEvent;
use headroom_core::usage::{PriceBook, event_cost};
use jiff::{SignedDuration, Timestamp};

pub const SPEND_LOOKBACK: SignedDuration = SignedDuration::from_hours(26);

#[must_use]
pub fn recent_spend(
    events: &[UsageEvent],
    prices: &dyn PriceBook,
    now: Timestamp,
) -> Vec<SpendPoint> {
    let since = now.checked_sub(SPEND_LOOKBACK).unwrap_or(Timestamp::MIN);
    spend_points(events.iter().filter(|event| event.at >= since), prices)
}

#[must_use]
pub fn spend_points<'a>(
    events: impl IntoIterator<Item = &'a UsageEvent>,
    prices: &dyn PriceBook,
) -> Vec<SpendPoint> {
    let mut points: Vec<SpendPoint> = events
        .into_iter()
        .map(|event| SpendPoint {
            at: event.at,
            cost: event_cost(event, prices),
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
