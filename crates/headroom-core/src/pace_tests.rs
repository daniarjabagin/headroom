use super::*;
use crate::quota::WindowId;

const FIVE_HOURS: SignedDuration = SignedDuration::from_hours(5);

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

fn session(used: f64, elapsed_mins: i64) -> QuotaWindow {
    window(used, SignedDuration::from_mins(elapsed_mins), FIVE_HOURS)
}

fn severity(window: &QuotaWindow) -> Severity {
    pace(window, now()).severity
}

#[test]
fn nearly_full_usage_is_spent() {
    for used in [99.6, 100.0, 130.0] {
        let result = pace(&session(used, 150), now());
        assert_eq!(result.severity, Severity::Spent, "used {used}");
        assert_eq!(result.projected, None);
        assert_eq!(result.runs_out_at, None);
    }
}

#[test]
fn spent_without_timing_is_still_spent() {
    let mut spent = session(100.0, 150);
    spent.resets_at = None;
    assert_eq!(severity(&spent), Severity::Spent);
}

#[test]
fn half_percent_remaining_is_not_spent() {
    assert_eq!(severity(&session(99.5, 150)), Severity::RunningOut);
}

#[test]
fn zero_usage_is_untracked_with_even_pace() {
    let result = pace(&session(0.0, 150), now());
    assert_eq!(result.severity, Severity::Untracked);
    assert_eq!(result.even_pace, Some(Percent::new(50.0)));
    assert_eq!(result.projected, None);
}

#[test]
fn missing_reset_or_period_is_untracked() {
    let mut no_reset = session(50.0, 150);
    no_reset.resets_at = None;
    let mut no_period = session(50.0, 150);
    no_period.period = None;
    let mut zero_period = session(50.0, 150);
    zero_period.period = Some(SignedDuration::ZERO);
    for window in [no_reset, no_period, zero_period] {
        let result = pace(&window, now());
        assert_eq!(result.severity, Severity::Untracked);
        assert_eq!(result.even_pace, None);
    }
}

#[test]
fn reset_in_past_or_now_is_untracked() {
    let mut past = session(50.0, 150);
    past.resets_at = Some(now() - SignedDuration::from_secs(1));
    let mut exactly_now = session(50.0, 150);
    exactly_now.resets_at = Some(now());
    assert_eq!(severity(&past), Severity::Untracked);
    assert_eq!(severity(&exactly_now), Severity::Untracked);
}

#[test]
fn young_window_uses_one_percent_of_period() {
    let young = window(50.0, SignedDuration::from_secs(179), FIVE_HOURS);
    let old_enough = window(0.5, SignedDuration::from_secs(180), FIVE_HOURS);
    assert_eq!(severity(&young), Severity::Untracked);
    assert_eq!(severity(&old_enough), Severity::Healthy);
}

#[test]
fn young_window_uses_sixty_seconds_minimum() {
    let hour = SignedDuration::from_hours(1);
    let young = window(0.5, SignedDuration::from_secs(59), hour);
    let old_enough = window(0.5, SignedDuration::from_secs(60), hour);
    assert_eq!(severity(&young), Severity::Untracked);
    assert_eq!(severity(&old_enough), Severity::Healthy);
}

#[test]
fn slow_burn_is_healthy_with_projection() {
    let result = pace(&session(40.0, 150), now());
    assert_eq!(result.severity, Severity::Healthy);
    assert_eq!(result.even_pace, Some(Percent::new(50.0)));
    assert_eq!(result.projected, Some(Percent::new(80.0)));
    assert_eq!(result.runs_out_at, None);
}

#[test]
fn projection_of_exactly_ninety_is_healthy() {
    assert_eq!(severity(&session(45.0, 150)), Severity::Healthy);
}

#[test]
fn small_usage_with_fast_burn_is_untracked() {
    let result = pace(&session(4.0, 6), now());
    assert_eq!(result.severity, Severity::Untracked);
    assert_eq!(result.projected, None);
}

#[test]
fn projection_up_to_hundred_is_close() {
    let close = pace(&session(48.0, 150), now());
    assert_eq!(close.severity, Severity::Close);
    assert_eq!(close.projected, Some(Percent::new(96.0)));
    assert_eq!(close.runs_out_at, None);
    assert_eq!(severity(&session(50.0, 150)), Severity::Close);
}

#[test]
fn fast_burn_runs_out_before_reset() {
    let result = pace(&session(60.0, 150), now());
    assert_eq!(result.severity, Severity::RunningOut);
    assert_eq!(result.projected, Some(Percent::new(120.0)));
    assert_eq!(
        result.runs_out_at,
        Some(now() + SignedDuration::from_secs(6_000))
    );
}

#[test]
fn minimum_tracked_usage_can_run_out() {
    let result = pace(&session(5.0, 6), now());
    assert_eq!(result.severity, Severity::RunningOut);
    assert_eq!(
        result.runs_out_at,
        Some(now() + SignedDuration::from_mins(114))
    );
}

fn tone_for(severity: Severity, used: f64) -> Tone {
    let pace = Pace {
        severity,
        even_pace: None,
        projected: None,
        runs_out_at: None,
    };
    tone(&session(used, 150), &pace)
}

#[test]
fn spent_and_running_out_are_critical_regardless_of_usage() {
    assert_eq!(tone_for(Severity::Spent, 100.0), Tone::Critical);
    assert_eq!(tone_for(Severity::RunningOut, 20.0), Tone::Critical);
}

#[test]
fn close_is_warning_regardless_of_usage() {
    assert_eq!(tone_for(Severity::Close, 10.0), Tone::Warning);
    assert_eq!(tone_for(Severity::Close, 95.0), Tone::Warning);
}

#[test]
fn healthy_is_good_regardless_of_usage() {
    assert_eq!(tone_for(Severity::Healthy, 10.0), Tone::Good);
    assert_eq!(tone_for(Severity::Healthy, 89.0), Tone::Good);
}

#[test]
fn untracked_at_ninety_used_is_critical() {
    assert_eq!(tone_for(Severity::Untracked, 90.0), Tone::Critical);
    assert_eq!(tone_for(Severity::Untracked, 95.0), Tone::Critical);
}

#[test]
fn untracked_at_eighty_used_is_warning() {
    assert_eq!(tone_for(Severity::Untracked, 80.0), Tone::Warning);
    assert_eq!(tone_for(Severity::Untracked, 89.9), Tone::Warning);
}

#[test]
fn untracked_below_eighty_used_is_good() {
    assert_eq!(tone_for(Severity::Untracked, 79.9), Tone::Good);
    assert_eq!(tone_for(Severity::Untracked, 0.0), Tone::Good);
}

#[test]
fn tone_without_data_is_neutral() {
    assert_eq!(Tone::default(), Tone::Neutral);
}

#[test]
fn tone_follows_pace_end_to_end() {
    let running = session(60.0, 150);
    assert_eq!(tone(&running, &pace(&running, now())), Tone::Critical);
    let healthy = session(40.0, 150);
    assert_eq!(tone(&healthy, &pace(&healthy, now())), Tone::Good);
    let young_heavy = window(85.0, SignedDuration::from_secs(10), FIVE_HOURS);
    assert_eq!(
        tone(&young_heavy, &pace(&young_heavy, now())),
        Tone::Warning
    );
}

#[test]
fn severity_serializes_snake_case() {
    assert_eq!(
        serde_json::to_string(&Severity::RunningOut).unwrap(),
        "\"running_out\""
    );
    assert_eq!(
        serde_json::to_string(&Tone::Warning).unwrap(),
        "\"warning\""
    );
}
