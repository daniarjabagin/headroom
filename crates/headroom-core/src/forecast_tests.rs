use super::*;
use crate::pace::{Tone, tone};
use crate::quota::WindowId;

const FIVE_HOURS: SignedDuration = SignedDuration::from_hours(5);
const WEEK: SignedDuration = SignedDuration::from_hours(7 * 24);

fn now() -> Timestamp {
    "2026-09-23T10:00:00Z".parse().unwrap()
}

fn window(used: f64, elapsed: SignedDuration, period: SignedDuration) -> QuotaWindow {
    QuotaWindow {
        id: WindowId::Session,
        label: "Session".into(),
        used: Percent::new(used),
        resets_at: Some(now() + (period - elapsed)),
        period: Some(period),
    }
}

fn session(used: f64) -> QuotaWindow {
    window(used, SignedDuration::from_mins(150), FIVE_HOURS)
}

fn ago(mins: i64, used: f64) -> UsageSample {
    UsageSample {
        at: now() - SignedDuration::from_mins(mins),
        used: Percent::new(used),
    }
}

fn run(window: &QuotaWindow, samples: &[UsageSample], live: bool) -> Pace {
    forecast(window, Activity { samples, live }, now())
}

fn bursty_week() -> (QuotaWindow, Vec<UsageSample>) {
    let week = window(20.0, SignedDuration::from_hours(72), WEEK);
    let mut samples = vec![ago(48 * 60, 14.0)];
    samples.extend((0..6_i32).map(|step| ago(i64::from(50 - step * 10), 15.0 + f64::from(step))));
    (week, samples)
}

fn burst_then_idle() -> Vec<UsageSample> {
    vec![
        ago(130, 20.0),
        ago(125, 30.0),
        ago(120, 40.0),
        ago(115, 50.0),
        ago(110, 60.0),
    ]
}

#[test]
fn a_burst_after_idle_days_runs_out_soon() {
    let (week, samples) = bursty_week();
    assert_eq!(pace(&week, now()).severity, Severity::Healthy);
    let result = run(&week, &samples, false);
    assert_eq!(result.basis, Some(Basis::Recent));
    assert_eq!(result.severity, Severity::RunningOut);
    assert_eq!(
        result.runs_out_at,
        Some(now() + SignedDuration::from_secs(48_000))
    );
    let projected = f64::from(result.projected.unwrap());
    assert!((projected - 596.0).abs() < 1e-9, "{projected}");
    assert_eq!(result.active_left, None);
    assert_eq!(tone(&week, &result, now()), Tone::Critical);
}

#[test]
fn a_burst_then_idle_is_paused_and_not_critical() {
    let busy = session(60.0);
    let result = run(&busy, &burst_then_idle(), false);
    assert_eq!(result.basis, Some(Basis::Paused));
    assert_eq!(result.severity, Severity::RunningOut);
    assert_eq!(result.runs_out_at, None);
    assert_eq!(result.projected, Some(Percent::new(120.0)));
    assert_eq!(result.active_left, Some(SignedDuration::from_mins(20)));
    assert_eq!(tone(&busy, &result, now()), Tone::Warning);
}

#[test]
fn live_activity_without_recent_steps_keeps_the_window_average() {
    let busy = session(60.0);
    let result = run(&busy, &burst_then_idle(), true);
    assert_eq!(result, pace(&busy, now()));
    assert_eq!(result.basis, Some(Basis::Window));
}

#[test]
fn integer_steps_need_three_rises_before_a_recent_rate() {
    let cases = [
        (vec![ago(0, 42.0)], Basis::Window),
        (
            vec![ago(6, 40.0), ago(3, 41.0), ago(0, 42.0)],
            Basis::Window,
        ),
        (
            vec![ago(9, 39.0), ago(6, 40.0), ago(3, 41.0), ago(0, 42.0)],
            Basis::Recent,
        ),
    ];
    for (samples, expected) in cases {
        let result = run(&session(42.0), &samples, false);
        assert_eq!(result.basis, Some(expected), "{samples:?}");
    }
}

#[test]
fn samples_from_an_earlier_window_are_ignored() {
    let fresh = window(12.0, SignedDuration::from_mins(60), FIVE_HOURS);
    let samples = [
        ago(80, 70.0),
        ago(75, 80.0),
        ago(70, 90.0),
        ago(65, 95.0),
        ago(3, 11.0),
        ago(0, 12.0),
    ];
    let result = run(&fresh, &samples, false);
    assert_eq!(result.basis, Some(Basis::Window));
    assert_eq!(result, pace(&fresh, now()));
}

#[test]
fn slow_recent_work_projects_below_the_window_average() {
    let steady = session(48.0);
    let samples = [ago(27, 45.0), ago(18, 46.0), ago(9, 47.0), ago(0, 48.0)];
    let result = run(&steady, &samples, false);
    assert_eq!(pace(&steady, now()).severity, Severity::Close);
    assert_eq!(result.basis, Some(Basis::Recent));
    assert_eq!(result.severity, Severity::Healthy);
    let projected = f64::from(result.projected.unwrap());
    assert!(
        (projected - 64.666_666_666_666_67).abs() < 1e-9,
        "{projected}"
    );
    assert_eq!(result.runs_out_at, None);
}

#[test]
fn untracked_and_spent_windows_have_no_basis() {
    let steps = [ago(3, 1.0), ago(2, 2.0), ago(1, 3.0), ago(0, 4.0)];
    let young = window(50.0, SignedDuration::from_mins(10), FIVE_HOURS);
    let cases = [
        (session(4.0), Severity::Untracked),
        (session(100.0), Severity::Spent),
        (young, Severity::Untracked),
        (session(0.0), Severity::Untracked),
    ];
    for (window, severity) in cases {
        let result = run(&window, &steps, false);
        assert_eq!(result.severity, severity, "{window:?}");
        assert_eq!(result.basis, None);
        assert_eq!(result.runs_out_at, None);
        assert_eq!(result.active_left, None);
    }
}

#[test]
fn paused_without_enough_steps_has_no_active_estimate() {
    let busy = session(60.0);
    let result = run(&busy, &[ago(60, 60.0)], false);
    assert_eq!(result.basis, Some(Basis::Paused));
    assert_eq!(result.active_left, None);
}

#[test]
fn a_healthy_idle_window_is_paused_too() {
    let calm = session(30.0);
    let result = run(&calm, &[ago(60, 30.0)], false);
    assert_eq!(result.basis, Some(Basis::Paused));
    assert_eq!(result.severity, Severity::Healthy);
}

#[test]
fn a_change_missing_from_history_is_not_idle() {
    let busy = session(60.0);
    let result = run(&busy, &[ago(60, 55.0)], false);
    assert_eq!(result.basis, Some(Basis::Window));
}
