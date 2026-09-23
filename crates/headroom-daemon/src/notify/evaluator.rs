use std::collections::BTreeSet;

use headroom_core::pace::{Severity, Tone, pace, tone};
use headroom_core::quota::QuotaWindow;
use jiff::{SignedDuration, Timestamp};
use serde::{Deserialize, Serialize};

const ALMOST_OUT_BELOW: f64 = 10.0;
const ALMOST_OUT_REARM_AT: f64 = 15.0;
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
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Observation {
    pub remaining: f64,
    pub severity: Severity,
    pub tone: Tone,
    pub resets_at: Option<Timestamp>,
    pub runs_out_at: Option<Timestamp>,
}

impl Observation {
    #[must_use]
    pub fn of(window: &QuotaWindow, now: Timestamp) -> Observation {
        let pace = pace(window, now);
        Observation {
            remaining: window.used.remaining().value(),
            severity: pace.severity,
            tone: tone(window, &pace, now),
            resets_at: window.resets_at,
            runs_out_at: pace.runs_out_at,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Evaluation {
    pub state: AlertState,
    pub alerts: Vec<Milestone>,
}

#[must_use]
pub fn evaluate(previous: Option<&AlertState>, observed: &Observation) -> Evaluation {
    let Some(previous) = previous else {
        return prime(observed);
    };
    let mut state = previous.clone();
    if has_reset(previous.resets_at, observed.resets_at) {
        state.fired.clear();
        state.reset_owed |= previous.tone >= Tone::Warning;
    }
    rearm(&mut state.fired, observed);
    let reached = reached(observed);
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

fn prime(observed: &Observation) -> Evaluation {
    Evaluation {
        state: AlertState {
            resets_at: observed.resets_at,
            tone: observed.tone,
            fired: reached(observed),
            reset_owed: false,
        },
        alerts: Vec::new(),
    }
}

fn has_reset(previous: Option<Timestamp>, current: Option<Timestamp>) -> bool {
    match (previous, current) {
        (Some(previous), Some(current)) => current.duration_since(previous) > RESET_TOLERANCE,
        _ => false,
    }
}

fn reached(observed: &Observation) -> BTreeSet<Milestone> {
    let mut reached = BTreeSet::new();
    if observed.remaining < ALMOST_OUT_BELOW {
        reached.insert(Milestone::AlmostOut);
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

fn rearm(fired: &mut BTreeSet<Milestone>, observed: &Observation) {
    if observed.remaining >= ALMOST_OUT_REARM_AT {
        fired.remove(&Milestone::AlmostOut);
    }
    if observed.severity < Severity::Close {
        fired.remove(&Milestone::CuttingItClose);
        fired.remove(&Milestone::WillRunOut);
    }
}

#[cfg(test)]
#[path = "evaluator_tests.rs"]
mod tests;
