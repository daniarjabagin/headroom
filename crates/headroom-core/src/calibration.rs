use jiff::Timestamp;

use crate::history::UsageSample;
use crate::units::{MicroUsd, Percent};

const MIN_RISE: f64 = 3.0;
const MIN_SPEND: MicroUsd = MicroUsd(100_000);
const ENOUGH_RISE: f64 = 20.0;
const REPORTED_STEP: f64 = 1.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SpendPoint {
    pub at: Timestamp,
    pub cost: MicroUsd,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Calibration {
    rise: f64,
    spend: MicroUsd,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Observed {
    pub used: Percent,
    pub changed_at: Timestamp,
    pub observed_at: Timestamp,
}

impl Calibration {
    #[must_use]
    pub fn percent_of(self, spend: MicroUsd) -> f64 {
        micros(spend) * self.rise / micros(self.spend)
    }

    #[must_use]
    pub fn rate(self, spend: &[SpendPoint], from: Timestamp, to: Timestamp) -> Option<f64> {
        let secs = to.duration_since(from).as_secs_f64();
        let rate = self.percent_of(spend_between(spend, from, to)) / secs;
        (secs > 0.0 && rate > 0.0).then_some(rate)
    }

    #[must_use]
    pub fn estimate_used(
        self,
        observed: Observed,
        spend: &[SpendPoint],
        now: Timestamp,
    ) -> Percent {
        let used = observed.used.value();
        let seen_at = observed.observed_at.max(observed.changed_at);
        let since_change = self.percent_of(spend_between(spend, observed.changed_at, now));
        let since_seen = self.percent_of(spend_between(spend, seen_at, now));
        let estimate = (used + since_change).min(used + REPORTED_STEP + since_seen);
        Percent::new(estimate.min(100.0).max(used))
    }
}

#[must_use]
pub fn calibrate(samples: &[UsageSample], spend: &[SpendPoint]) -> Option<Calibration> {
    let mut rise = 0.0;
    let mut cost = MicroUsd::ZERO;
    for pair in samples.windows(2).rev() {
        let [earlier, later] = pair else {
            break;
        };
        rise += (later.used.value() - earlier.used.value()).max(0.0);
        cost += spend_between(spend, earlier.at, later.at);
        if rise >= ENOUGH_RISE {
            break;
        }
    }
    (rise >= MIN_RISE && cost >= MIN_SPEND).then_some(Calibration { rise, spend: cost })
}

#[must_use]
pub fn spend_between(spend: &[SpendPoint], from: Timestamp, to: Timestamp) -> MicroUsd {
    let start = spend.partition_point(|point| point.at <= from);
    let end = spend.partition_point(|point| point.at <= to);
    spend
        .get(start..end)
        .unwrap_or_default()
        .iter()
        .map(|point| point.cost)
        .sum()
}

#[allow(
    clippy::cast_precision_loss,
    reason = "a day of micro-USD stays far below 2^53; the result only scales a percent"
)]
fn micros(spend: MicroUsd) -> f64 {
    spend.0 as f64
}

#[cfg(test)]
#[path = "calibration_tests.rs"]
mod tests;
