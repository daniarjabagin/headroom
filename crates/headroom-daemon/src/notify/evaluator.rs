use std::collections::BTreeSet;

use headroom_core::pace::{self, Basis, Pace, Severity, Tone, tone};
use headroom_core::quota::QuotaWindow;
use jiff::{SignedDuration, Timestamp};
use serde::{Deserialize, Serialize};

const ALMOST_OUT_REARM_MARGIN: f64 = 5.0;
const RESET_TOLERANCE: SignedDuration = SignedDuration::from_secs(1);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Milestone {
    Reset,
    WillRunOut,
    CuttingItClose,
    AlmostOut,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AlertState {
    pub resets_at: Option<Timestamp>,
    pub tone: Tone,
    pub fired: BTreeSet<Milestone>,
    pub reset_owed: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub recent_seen: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Observation {
    pub remaining: f64,
    pub severity: Severity,
    pub tone: Tone,
    pub resets_at: Option<Timestamp>,
    pub runs_out_at: Option<Timestamp>,
    #[serde(default)]
    pub paused: bool,
    #[serde(default)]
    pub recent: bool,
    #[serde(default = "untracked")]
    pub window_severity: Severity,
}

impl Observation {
    #[must_use]
    pub fn of(window: &QuotaWindow, pace: &Pace, now: Timestamp) -> Observation {
        Observation {
            remaining: window.used.remaining().value(),
            severity: pace.severity,
            tone: tone(window, pace, now),
            resets_at: window.resets_at,
            runs_out_at: pace.runs_out_at,
            paused: pace.basis == Some(Basis::Paused),
            recent: pace.basis == Some(Basis::Recent),
            window_severity: pace::pace(window, now).severity,
        }
    }
}

fn untracked() -> Severity {
    Severity::Untracked
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Evaluation {
    pub state: AlertState,
    pub alerts: Vec<Milestone>,
}

#[must_use]
pub fn evaluate(
    previous: Option<&AlertState>,
    observed: &Observation,
    threshold: u8,
) -> Evaluation {
    let Some(previous) = previous else {
        return prime(observed, threshold);
    };
    let mut state = previous.clone();
    if has_reset(previous.resets_at, observed.resets_at) {
        state.fired.clear();
        state.recent_seen = false;
        state.reset_owed |= previous.tone >= Tone::Warning;
    }
    state.recent_seen |= observed.recent;
    rearm(&mut state, observed, threshold);
    let reached = reached(observed, threshold);
    let mut alerts: Vec<Milestone> = reached
        .iter()
        .copied()
        .filter(|m| state.fired.insert(*m) && !suppressed(*m, &reached))
        .collect();
    if std::mem::take(&mut state.reset_owed) {
        alerts.push(Milestone::Reset);
    }
    alerts.sort();
    state.resets_at = observed.resets_at;
    state.tone = observed.tone;
    Evaluation { state, alerts }
}

pub fn rollback(state: &mut AlertState, milestone: Milestone) {
    match milestone {
        Milestone::Reset => state.reset_owed = true,
        other => {
            state.fired.remove(&other);
        }
    }
}

fn prime(observed: &Observation, threshold: u8) -> Evaluation {
    Evaluation {
        state: AlertState {
            resets_at: observed.resets_at,
            tone: observed.tone,
            fired: reached(observed, threshold),
            reset_owed: false,
            recent_seen: observed.recent,
        },
        alerts: Vec::new(),
    }
}

pub(super) fn has_reset(previous: Option<Timestamp>, current: Option<Timestamp>) -> bool {
    match (previous, current) {
        (Some(previous), Some(current)) => current.duration_since(previous) > RESET_TOLERANCE,
        _ => false,
    }
}

fn reached(observed: &Observation, threshold: u8) -> BTreeSet<Milestone> {
    let mut reached = BTreeSet::new();
    if observed.remaining < f64::from(threshold) {
        reached.insert(Milestone::AlmostOut);
    }
    if observed.paused {
        return reached;
    }
    if observed.severity >= Severity::Close {
        reached.insert(Milestone::CuttingItClose);
    }
    if observed.severity >= Severity::RunningOut {
        reached.insert(Milestone::WillRunOut);
    }
    reached
}

fn suppressed(milestone: Milestone, reached: &BTreeSet<Milestone>) -> bool {
    milestone == Milestone::CuttingItClose && reached.contains(&Milestone::WillRunOut)
}

fn rearm(state: &mut AlertState, observed: &Observation, threshold: u8) {
    if observed.remaining >= f64::from(threshold) + ALMOST_OUT_REARM_MARGIN {
        state.fired.remove(&Milestone::AlmostOut);
    }
    if pace_calmed(state.recent_seen, observed) {
        state.fired.remove(&Milestone::CuttingItClose);
        state.fired.remove(&Milestone::WillRunOut);
    }
}

fn pace_calmed(recent_seen: bool, observed: &Observation) -> bool {
    let calm = observed.severity < Severity::Close && observed.window_severity < Severity::Close;
    calm && !observed.paused && (observed.recent || !recent_seen)
}

#[cfg(test)]
#[path = "evaluator_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "evaluator_forecast_tests.rs"]
mod forecast_tests;
