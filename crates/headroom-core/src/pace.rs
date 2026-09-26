use jiff::{SignedDuration, Timestamp};
use serde::{Deserialize, Serialize};

use crate::quota::QuotaWindow;
use crate::units::Percent;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Pace {
    pub severity: Severity,
    pub even_pace: Option<Percent>,
    pub projected: Option<Percent>,
    pub runs_out_at: Option<Timestamp>,
    pub basis: Option<Basis>,
    pub active_left: Option<SignedDuration>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Basis {
    Recent,
    Window,
    Paused,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Untracked,
    Healthy,
    Close,
    RunningOut,
    Spent,
}

#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum Tone {
    #[default]
    Neutral,
    Good,
    Warning,
    Critical,
}

const HEALTHY_PROJECTION: f64 = 90.0;
const CLOSE_PROJECTION: f64 = 100.0;
const MIN_TRACKED_USED: f64 = 5.0;
const MIN_ELAPSED: SignedDuration = SignedDuration::from_secs(60);
const YOUNG_SHARE_PERCENT: i32 = 15;
const YOUNG_CAP: SignedDuration = SignedDuration::from_hours(24);
const IMMINENT_SHARE_PERCENT: i32 = 15;
const IMMINENT_FLOOR: SignedDuration = SignedDuration::from_hours(1);
const WARNING_USED: f64 = 80.0;
const CRITICAL_USED: f64 = 90.0;

#[must_use]
pub fn pace(window: &QuotaWindow, now: Timestamp) -> Pace {
    let timing = Timing::of(window, now);
    let even_pace = timing.map(|t| Percent::new(t.progress * 100.0));
    let (severity, projected, runs_out_at) = project(window.used, timing, now);
    Pace {
        severity,
        even_pace,
        projected,
        runs_out_at,
        basis: is_tracked(severity).then_some(Basis::Window),
        active_left: None,
    }
}

#[must_use]
pub fn is_tracked(severity: Severity) -> bool {
    matches!(
        severity,
        Severity::Healthy | Severity::Close | Severity::RunningOut
    )
}

#[must_use]
pub fn tone(window: &QuotaWindow, pace: &Pace, now: Timestamp) -> Tone {
    match pace.severity {
        Severity::Spent => Tone::Critical,
        Severity::RunningOut => running_out_tone(window, pace, now),
        Severity::Close => Tone::Warning,
        Severity::Healthy => Tone::Good,
        Severity::Untracked => tone_by_usage(window.used),
    }
}

fn running_out_tone(window: &QuotaWindow, pace: &Pace, now: Timestamp) -> Tone {
    let imminent = pace
        .runs_out_at
        .is_some_and(|at| at.duration_since(now) <= imminent_within(window.period));
    if imminent || window.used.value() >= CRITICAL_USED {
        Tone::Critical
    } else {
        Tone::Warning
    }
}

fn imminent_within(period: Option<SignedDuration>) -> SignedDuration {
    period
        .map_or(SignedDuration::ZERO, |p| {
            share_of(p, IMMINENT_SHARE_PERCENT)
        })
        .max(IMMINENT_FLOOR)
}

fn young_until(period: SignedDuration) -> SignedDuration {
    share_of(period, YOUNG_SHARE_PERCENT)
        .min(YOUNG_CAP)
        .max(MIN_ELAPSED)
}

fn share_of(period: SignedDuration, percent: i32) -> SignedDuration {
    (period / 100)
        .checked_mul(percent)
        .unwrap_or(SignedDuration::MAX)
}

fn tone_by_usage(used: Percent) -> Tone {
    let used = used.value();
    if used >= CRITICAL_USED {
        Tone::Critical
    } else if used >= WARNING_USED {
        Tone::Warning
    } else {
        Tone::Good
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct Timing {
    pub(crate) start: Timestamp,
    pub(crate) reset: Timestamp,
    pub(crate) period: SignedDuration,
    elapsed: SignedDuration,
    progress: f64,
    pub(crate) young: bool,
}

impl Timing {
    pub(crate) fn of(window: &QuotaWindow, now: Timestamp) -> Option<Timing> {
        let reset = window.resets_at?;
        let period = window.period.filter(SignedDuration::is_positive)?;
        if reset <= now {
            return None;
        }
        let start = reset.checked_sub(period).ok()?;
        let elapsed = now.duration_since(start).max(SignedDuration::ZERO);
        let progress = (elapsed.as_secs_f64() / period.as_secs_f64()).clamp(0.0, 1.0);
        let young = elapsed < young_until(period);
        Some(Timing {
            start,
            reset,
            period,
            elapsed,
            progress,
            young,
        })
    }

    fn runs_out_at(self, used: f64, now: Timestamp) -> Option<Timestamp> {
        let secs = self.elapsed.as_secs_f64() * 100.0 / used;
        let offset = SignedDuration::try_from_secs_f64(secs).ok()?;
        let at = self.start.checked_add(offset).ok()?;
        (now < at && at < self.reset).then_some(at)
    }
}

type Projection = (Severity, Option<Percent>, Option<Timestamp>);

fn project(used: Percent, timing: Option<Timing>, now: Timestamp) -> Projection {
    let untracked = (Severity::Untracked, None, None);
    if is_spent(used) {
        return (Severity::Spent, None, None);
    }
    let used = used.value();
    let Some(timing) = timing.filter(|t| used > 0.0 && !t.young) else {
        return untracked;
    };
    let projected = used / timing.progress;
    let Some(severity) = classify(used, projected) else {
        return untracked;
    };
    let runs_out_at = timing.runs_out_at(used, now);
    (severity, Some(Percent::new(projected)), runs_out_at)
}

pub(crate) fn classify(used: f64, projected: f64) -> Option<Severity> {
    if projected <= HEALTHY_PROJECTION {
        Some(Severity::Healthy)
    } else if used < MIN_TRACKED_USED {
        None
    } else if projected <= CLOSE_PROJECTION {
        Some(Severity::Close)
    } else {
        Some(Severity::RunningOut)
    }
}

pub(crate) fn is_spent(used: Percent) -> bool {
    used.remaining().value().round() <= 0.0
}

#[cfg(test)]
#[path = "pace_tests.rs"]
mod tests;
