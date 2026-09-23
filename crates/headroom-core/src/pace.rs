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
    }
}

#[must_use]
pub fn tone(window: &QuotaWindow, pace: &Pace) -> Tone {
    match pace.severity {
        Severity::Spent | Severity::RunningOut => Tone::Critical,
        Severity::Close => Tone::Warning,
        Severity::Healthy => Tone::Good,
        Severity::Untracked => tone_by_usage(window.used),
    }
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
struct Timing {
    start: Timestamp,
    reset: Timestamp,
    elapsed: SignedDuration,
    progress: f64,
    young: bool,
}

impl Timing {
    fn of(window: &QuotaWindow, now: Timestamp) -> Option<Timing> {
        let reset = window.resets_at?;
        let period = window.period.filter(SignedDuration::is_positive)?;
        if reset <= now {
            return None;
        }
        let start = reset.checked_sub(period).ok()?;
        let elapsed = now.duration_since(start).max(SignedDuration::ZERO);
        let progress = (elapsed.as_secs_f64() / period.as_secs_f64()).clamp(0.0, 1.0);
        let young = elapsed < MIN_ELAPSED.max(period / 100);
        Some(Timing {
            start,
            reset,
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
    let severity = if projected <= HEALTHY_PROJECTION {
        Severity::Healthy
    } else if used < MIN_TRACKED_USED {
        return untracked;
    } else if projected <= CLOSE_PROJECTION {
        Severity::Close
    } else {
        Severity::RunningOut
    };
    let runs_out_at = timing.runs_out_at(used, now);
    (severity, Some(Percent::new(projected)), runs_out_at)
}

fn is_spent(used: Percent) -> bool {
    used.remaining().value().round() <= 0.0
}

#[cfg(test)]
#[path = "pace_tests.rs"]
mod tests;
