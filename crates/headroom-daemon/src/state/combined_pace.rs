use headroom_core::pace::{Basis, Severity, Tone, is_tracked};

use super::payload::{PaceView, WindowView};

const HEALTHY_PROJECTION: f64 = 90.0;
const CLOSE_PROJECTION: f64 = 100.0;
const MIN_TRACKED_USED: f64 = 5.0;
const WARNING_USED: f64 = 80.0;
const CRITICAL_USED: f64 = 90.0;

#[derive(Debug, Clone, Copy)]
struct Totals {
    capacity: f64,
    used: f64,
    remaining: f64,
    projected: Option<f64>,
    even_pace: Option<f64>,
}

impl Totals {
    fn of(segments: &[&WindowView]) -> Totals {
        let tracked = segments.iter().any(|w| w.pace.projected_percent.is_some());
        let projected: f64 = segments.iter().map(|w| projected_used(w)).sum();
        Totals {
            capacity: capacity(segments.len()),
            used: segments.iter().map(|w| w.used_percent).sum(),
            remaining: segments.iter().map(|w| w.remaining_percent).sum(),
            projected: tracked.then_some(projected),
            even_pace: segments.iter().map(|w| w.pace.even_pace_percent).sum(),
        }
    }

    fn share(self, value: f64) -> f64 {
        value / self.capacity * 100.0
    }
}

#[must_use]
pub fn combined_pace(segments: &[&WindowView]) -> (PaceView, Tone) {
    if let [only] = segments {
        return (only.pace.clone(), only.tone);
    }
    let totals = Totals::of(segments);
    let severity = severity(totals);
    let basis = is_tracked(severity).then(|| basis(segments));
    let pace = PaceView {
        severity,
        even_pace_percent: totals.even_pace,
        projected_percent: totals.projected.filter(|_| is_tracked(severity)),
        spare_percent: spare(totals, severity),
        runs_out_at: None,
        basis,
        active_left_seconds: active_left(segments, basis),
    };
    (pace, tone(totals, severity))
}

fn basis(segments: &[&WindowView]) -> Basis {
    let bases = || segments.iter().filter_map(|w| w.pace.basis);
    if bases().any(|b| b == Basis::Recent) {
        Basis::Recent
    } else if all_paused(segments) {
        Basis::Paused
    } else {
        Basis::Window
    }
}

fn all_paused(segments: &[&WindowView]) -> bool {
    let mut members = segments
        .iter()
        .filter(|w| w.pace.severity != Severity::Spent)
        .peekable();
    members.peek().is_some() && members.all(|w| w.pace.basis == Some(Basis::Paused))
}

fn active_left(segments: &[&WindowView], basis: Option<Basis>) -> Option<u64> {
    if basis != Some(Basis::Paused) {
        return None;
    }
    segments
        .iter()
        .map(|w| match w.pace.severity {
            Severity::Spent => Some(0),
            _ => w.pace.active_left_seconds,
        })
        .try_fold(0_u64, |total, left| total.checked_add(left?))
}

fn projected_used(window: &WindowView) -> f64 {
    window.pace.projected_percent.unwrap_or(window.used_percent)
}

fn capacity(accounts: usize) -> f64 {
    f64::from(u32::try_from(accounts).unwrap_or(u32::MAX)) * 100.0
}

fn severity(totals: Totals) -> Severity {
    if totals.remaining.round() <= 0.0 {
        return Severity::Spent;
    }
    let Some(projected) = totals.projected.map(|p| totals.share(p)) else {
        return Severity::Untracked;
    };
    if projected <= HEALTHY_PROJECTION {
        Severity::Healthy
    } else if totals.share(totals.used) < MIN_TRACKED_USED {
        Severity::Untracked
    } else if projected <= CLOSE_PROJECTION {
        Severity::Close
    } else {
        Severity::RunningOut
    }
}

fn spare(totals: Totals, severity: Severity) -> Option<f64> {
    let on_track = matches!(severity, Severity::Healthy | Severity::Close);
    totals
        .projected
        .filter(|_| on_track)
        .map(|projected| (totals.capacity - projected).max(0.0))
}

fn tone(totals: Totals, severity: Severity) -> Tone {
    let used = totals.share(totals.used);
    match severity {
        Severity::Spent => Tone::Critical,
        Severity::RunningOut if used >= CRITICAL_USED => Tone::Critical,
        Severity::RunningOut | Severity::Close => Tone::Warning,
        Severity::Healthy => Tone::Good,
        Severity::Untracked => tone_by_usage(used),
    }
}

fn tone_by_usage(used: f64) -> Tone {
    if used >= CRITICAL_USED {
        Tone::Critical
    } else if used >= WARNING_USED {
        Tone::Warning
    } else {
        Tone::Good
    }
}

#[cfg(test)]
#[path = "combined_pace_tests.rs"]
mod tests;
