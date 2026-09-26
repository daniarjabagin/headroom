use jiff::{SignedDuration, Timestamp};

use crate::history::{UsageSample, current_samples};
use crate::pace::{Basis, Pace, Severity, Timing, classify, is_spent, is_tracked, pace};
use crate::pace_rate::{Cadence, last_active_rate, recent_rate};
use crate::quota::QuotaWindow;
use crate::units::Percent;

const RECENT_MAX_PERIOD: SignedDuration = SignedDuration::from_hours(24);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Liveness {
    Unknown,
    Live,
    Idle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Signal {
    pub liveness: Liveness,
    pub poll_interval: SignedDuration,
}

#[derive(Debug, Clone, Copy)]
pub struct Activity<'a> {
    pub samples: &'a [UsageSample],
    pub signal: Signal,
    pub observed_at: Timestamp,
}

#[must_use]
pub fn forecast(window: &QuotaWindow, activity: Activity<'_>, now: Timestamp) -> Pace {
    let average = pace(window, now);
    let Some(timing) = Timing::of(window, now).filter(|t| !t.young) else {
        return average;
    };
    if is_spent(window.used) || window.used.value() <= 0.0 {
        return average;
    }
    let samples = current_samples(activity.samples, window);
    let cadence = Cadence::of(timing.period, activity.signal.poll_interval);
    if is_paused(window, samples, cadence, activity) {
        return paused(window, average, samples, cadence);
    }
    if timing.period > RECENT_MAX_PERIOD {
        return average;
    }
    match recent_rate(samples, cadence, now) {
        Some(rate) => recent(window, &average, timing, rate, now),
        None => average,
    }
}

fn is_paused(
    window: &QuotaWindow,
    samples: &[UsageSample],
    cadence: Cadence,
    activity: Activity<'_>,
) -> bool {
    let idle = activity.signal.liveness == Liveness::Idle;
    let unchanged = samples.last().is_some_and(|last| last.used == window.used);
    idle && unchanged && cadence.is_idle(samples, activity.observed_at)
}

fn paused(window: &QuotaWindow, average: Pace, samples: &[UsageSample], cadence: Cadence) -> Pace {
    if !is_tracked(average.severity) {
        return average;
    }
    let active_left = last_active_rate(samples, cadence)
        .and_then(|rate| duration_at(window.used.remaining().value(), rate));
    Pace {
        runs_out_at: None,
        basis: Some(Basis::Paused),
        active_left,
        ..average
    }
}

fn recent(window: &QuotaWindow, average: &Pace, timing: Timing, rate: f64, now: Timestamp) -> Pace {
    let used = window.used.value();
    let left = timing.reset.duration_since(now).as_secs_f64();
    let projected = used + rate * left;
    let Some(severity) = classify(used, projected) else {
        return Pace {
            severity: Severity::Untracked,
            projected: None,
            runs_out_at: None,
            basis: None,
            ..*average
        };
    };
    let runs_out_at = duration_at(window.used.remaining().value(), rate)
        .and_then(|span| now.checked_add(span).ok())
        .filter(|at| *at < timing.reset);
    Pace {
        severity,
        even_pace: average.even_pace,
        projected: Some(Percent::new(projected)),
        runs_out_at,
        basis: Some(Basis::Recent),
        active_left: None,
    }
}

fn duration_at(remaining: f64, rate: f64) -> Option<SignedDuration> {
    SignedDuration::try_from_secs_f64(remaining / rate).ok()
}

#[cfg(test)]
#[path = "forecast_tests.rs"]
mod tests;
