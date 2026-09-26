use headroom_core::forecast::{Activity, Liveness, Signal, forecast};
use headroom_core::history::UsageSample;
use headroom_core::units::Percent;

use super::*;
use crate::testing::{session, ts};

const RESET: &str = "2026-09-23T12:30:00Z";
const NEXT_RESET: &str = "2026-09-23T17:30:00Z";
const THRESHOLD: u8 = 10;

fn sample(at: &str, used: f64) -> UsageSample {
    UsageSample {
        at: ts(at),
        used: Percent::new(used),
    }
}

fn observe_window(
    window: &QuotaWindow,
    samples: &[UsageSample],
    live: bool,
    at: &str,
) -> Observation {
    let now = ts(at);
    let liveness = if live { Liveness::Live } else { Liveness::Idle };
    let activity = Activity {
        samples,
        signal: Signal {
            liveness,
            poll_interval: SignedDuration::from_mins(5),
        },
        observed_at: now,
    };
    Observation::of(window, &forecast(window, activity, now), now)
}

fn observe(samples: &[UsageSample], live: bool, at: &str) -> Observation {
    let used = samples.last().map_or(0.0, |s| f64::from(s.used));
    observe_window(&session(used, RESET), samples, live, at)
}

fn first_burst() -> Vec<UsageSample> {
    vec![
        sample("2026-09-23T09:40:00Z", 20.0),
        sample("2026-09-23T09:51:00Z", 25.0),
        sample("2026-09-23T09:54:00Z", 30.0),
        sample("2026-09-23T09:57:00Z", 35.0),
        sample("2026-09-23T10:00:00Z", 40.0),
    ]
}

fn after_first_burst() -> AlertState {
    let history = first_burst();
    let primed = evaluate(
        None,
        &observe(&history[..1], true, "2026-09-23T09:40:00Z"),
        THRESHOLD,
    );
    let burst = observe(&history, true, "2026-09-23T10:00:00Z");
    assert!(burst.recent);
    assert_eq!(burst.severity, Severity::RunningOut);
    let first = evaluate(Some(&primed.state), &burst, THRESHOLD);
    assert_eq!(first.alerts, vec![Milestone::WillRunOut]);
    first.state
}

#[test]
fn a_fallback_to_the_window_average_between_bursts_does_not_rearm_pace_alerts() {
    let mut history = first_burst();
    let flat = observe(&history, true, "2026-09-23T10:15:00Z");
    assert!(!flat.recent);
    assert_eq!(flat.severity, Severity::Healthy);
    let quiet = evaluate(Some(&after_first_burst()), &flat, THRESHOLD);
    assert!(quiet.alerts.is_empty());
    history.extend([
        sample("2026-09-23T10:17:00Z", 42.0),
        sample("2026-09-23T10:20:00Z", 45.0),
        sample("2026-09-23T10:23:00Z", 48.0),
        sample("2026-09-23T10:26:00Z", 51.0),
    ]);
    let again = observe(&history, true, "2026-09-23T10:26:00Z");
    assert_eq!(again.severity, Severity::RunningOut);
    let second = evaluate(Some(&quiet.state), &again, THRESHOLD);
    assert!(second.alerts.is_empty(), "{:?}", second.alerts);
}

#[test]
fn a_calm_recent_rate_rearms_pace_alerts() {
    let mut calm = observe(&first_burst(), true, "2026-09-23T10:00:00Z");
    calm.severity = Severity::Healthy;
    let rearmed = evaluate(Some(&after_first_burst()), &calm, THRESHOLD);
    assert!(!rearmed.state.fired.contains(&Milestone::WillRunOut));
}

#[test]
fn a_window_reset_rearms_pace_alerts_and_forgets_the_recent_basis() {
    let fresh = session(12.0, NEXT_RESET);
    let reset = observe_window(&fresh, &[], true, "2026-09-23T12:31:00Z");
    let evaluation = evaluate(Some(&after_first_burst()), &reset, THRESHOLD);
    assert!(evaluation.state.fired.is_empty());
    assert!(!evaluation.state.recent_seen);
}

#[test]
fn a_paused_forecast_marks_the_observation() {
    let window = session(60.0, RESET);
    let samples = [sample("2026-09-23T08:10:00Z", 60.0)];
    let observed = observe_window(&window, &samples, false, "2026-09-23T10:00:00Z");
    assert!(observed.paused);
    assert_eq!(observed.severity, Severity::RunningOut);
    assert_eq!(observed.runs_out_at, None);
    assert_eq!(observed.tone, Tone::Warning);
    let fired = evaluate(None, &observed, THRESHOLD).state.fired;
    assert!(fired.is_empty());
}
