use jiff::{SignedDuration, Timestamp};
use serde::{Deserialize, Serialize};

use crate::quota::{LimitsSnapshot, LimitsSource, QuotaWindow};
use crate::units::Percent;

pub const SAMPLE_RETENTION: SignedDuration = SignedDuration::from_hours(8 * 24);

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct UsageSample {
    pub at: Timestamp,
    pub used: Percent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SampleStep {
    Append,
    Restart,
}

#[must_use]
pub fn observed_at(snapshot: &LimitsSnapshot) -> Timestamp {
    match snapshot.source {
        LimitsSource::Live => snapshot.fetched_at,
        LimitsSource::LocalLog { observed_at } => observed_at,
    }
}

#[must_use]
pub fn sample_step(
    last: Option<&UsageSample>,
    window: &QuotaWindow,
    at: Timestamp,
) -> Option<SampleStep> {
    let Some(last) = last else {
        return Some(SampleStep::Append);
    };
    if starts_after(window, last.at) {
        Some(SampleStep::Restart)
    } else if at <= last.at {
        None
    } else if window.used < last.used {
        Some(SampleStep::Restart)
    } else if window.used == last.used {
        None
    } else {
        Some(SampleStep::Append)
    }
}

#[must_use]
pub fn current_samples<'a>(samples: &'a [UsageSample], window: &QuotaWindow) -> &'a [UsageSample] {
    let first = samples
        .iter()
        .enumerate()
        .rev()
        .find(|(index, sample)| breaks_window(samples, *index, sample, window))
        .map_or(0, |(index, _)| index + 1);
    samples.get(first..).unwrap_or_default()
}

#[must_use]
pub fn is_retained(sample: &UsageSample, now: Timestamp) -> bool {
    now.duration_since(sample.at) <= SAMPLE_RETENTION
}

fn breaks_window(
    samples: &[UsageSample],
    index: usize,
    sample: &UsageSample,
    window: &QuotaWindow,
) -> bool {
    let next = samples.get(index + 1).map_or(window.used, |next| next.used);
    sample.used > next || starts_after(window, sample.at)
}

fn starts_after(window: &QuotaWindow, at: Timestamp) -> bool {
    window_start(window).is_some_and(|start| at < start)
}

fn window_start(window: &QuotaWindow) -> Option<Timestamp> {
    let period = window.period.filter(SignedDuration::is_positive)?;
    window.resets_at?.checked_sub(period).ok()
}

#[cfg(test)]
#[path = "history_tests.rs"]
mod tests;
