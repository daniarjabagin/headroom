use jiff::{SignedDuration, Timestamp};

use crate::history::UsageSample;

const IDLE_DIVISOR: i32 = 30;
const IDLE_FLOOR: SignedDuration = SignedDuration::from_mins(10);
const IDLE_CAP: SignedDuration = SignedDuration::from_hours(2);
const LOOKBACK_DIVISOR: i32 = 6;
const LOOKBACK_FLOOR: SignedDuration = SignedDuration::from_mins(30);
const LOOKBACK_CAP: SignedDuration = SignedDuration::from_hours(4);
const MIN_STEPS: u32 = 3;
const POLLS_BEFORE_IDLE: i32 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Cadence {
    pub(crate) idle_after: SignedDuration,
    lookback: SignedDuration,
}

impl Cadence {
    pub(crate) fn of(period: SignedDuration, poll_interval: SignedDuration) -> Cadence {
        let polls = poll_interval
            .checked_mul(POLLS_BEFORE_IDLE)
            .unwrap_or(SignedDuration::MAX);
        Cadence {
            idle_after: (period / IDLE_DIVISOR)
                .clamp(IDLE_FLOOR, IDLE_CAP)
                .max(polls),
            lookback: (period / LOOKBACK_DIVISOR).clamp(LOOKBACK_FLOOR, LOOKBACK_CAP),
        }
    }

    pub(crate) fn lookback(self) -> SignedDuration {
        self.lookback
    }

    pub(crate) fn is_idle(self, samples: &[UsageSample], now: Timestamp) -> bool {
        samples
            .last()
            .is_some_and(|last| now.duration_since(last.at) > self.idle_after)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Span {
    rise: f64,
    secs: f64,
    steps: u32,
}

impl Span {
    fn rate(self) -> f64 {
        self.rise / self.secs
    }

    fn step_secs(self) -> f64 {
        self.secs / f64::from(self.steps)
    }
}

pub(crate) fn last_active_rate(samples: &[UsageSample], cadence: Cadence) -> Option<f64> {
    latest_span(samples, cadence).map(Span::rate)
}

pub(crate) fn recent_rate(
    samples: &[UsageSample],
    cadence: Cadence,
    now: Timestamp,
) -> Option<f64> {
    if cadence.is_idle(samples, now) {
        return None;
    }
    let tail = now.duration_since(samples.last()?.at).as_secs_f64();
    let span = latest_span(samples, cadence)?;
    let overdue = (tail - span.step_secs()).max(0.0);
    Some(span.rise / (span.secs + overdue))
}

fn latest_span(samples: &[UsageSample], cadence: Cadence) -> Option<Span> {
    let mut span = Span {
        rise: 0.0,
        secs: 0.0,
        steps: 0,
    };
    for pair in samples.windows(2).rev() {
        let [earlier, later] = pair else {
            break;
        };
        let gap = later.at.duration_since(earlier.at);
        if gap > cadence.idle_after || !gap.is_positive() {
            break;
        }
        let rise = later.used.value() - earlier.used.value();
        span.rise += rise.max(0.0);
        span.secs += gap.as_secs_f64();
        span.steps += u32::from(rise > 0.0);
        if span.secs >= cadence.lookback.as_secs_f64() {
            break;
        }
    }
    (span.steps >= MIN_STEPS && span.rise > 0.0).then_some(span)
}

#[cfg(test)]
#[path = "pace_rate_tests.rs"]
mod tests;
