use super::*;
use crate::quota::WindowId;

const FIVE_HOURS: SignedDuration = SignedDuration::from_hours(5);
const WEEK: SignedDuration = SignedDuration::from_hours(7 * 24);
const MONTH: SignedDuration = SignedDuration::from_hours(30 * 24);

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
fn young_window_uses_fifteen_percent_of_period() {
    let young = window(50.0, SignedDuration::from_secs(2_699), FIVE_HOURS);
    let old_enough = window(10.0, SignedDuration::from_secs(2_700), FIVE_HOURS);
    assert_eq!(severity(&young), Severity::Untracked);
    assert_eq!(severity(&old_enough), Severity::Healthy);
}

#[test]
fn young_window_is_capped_at_a_day() {
    let young = window(10.0, SignedDuration::from_secs(86_399), WEEK);
    let old_enough = window(10.0, SignedDuration::from_hours(24), WEEK);
    assert_eq!(severity(&young), Severity::Untracked);
    assert_eq!(severity(&old_enough), Severity::Healthy);
}

#[test]
fn young_window_uses_sixty_seconds_minimum() {
    let five_minutes = SignedDuration::from_mins(5);
    let young = window(0.5, SignedDuration::from_secs(59), five_minutes);
    let old_enough = window(0.5, SignedDuration::from_secs(60), five_minutes);
    assert_eq!(severity(&young), Severity::Untracked);
    assert_eq!(severity(&old_enough), Severity::Healthy);
}

#[test]
fn early_weekly_burst_is_untracked_and_good() {
    let early = window(13.0, SignedDuration::from_hours(18), WEEK);
    let result = pace(&early, now());
    assert_eq!(result.severity, Severity::Untracked);
    assert_eq!(result.projected, None);
    assert_eq!(tone(&early, &result, now()), Tone::Good);
}

#[test]
fn settled_weekly_window_is_healthy() {
    let settled = window(13.0, SignedDuration::from_hours(30), WEEK);
    let result = pace(&settled, now());
    assert_eq!(result.severity, Severity::Healthy);
    let projected = f64::from(result.projected.unwrap());
    assert!((projected - 72.8).abs() < 1e-9, "{projected}");
    assert_eq!(tone(&settled, &result, now()), Tone::Good);
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
    let result = pace(&window(4.0, SignedDuration::from_hours(25), MONTH), now());
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
    let result = pace(&window(5.0, SignedDuration::from_hours(25), MONTH), now());
    assert_eq!(result.severity, Severity::RunningOut);
    assert_eq!(
        result.runs_out_at,
        Some(now() + SignedDuration::from_hours(475))
    );
}

fn tone_for(severity: Severity, used: f64, runs_out_in: Option<SignedDuration>) -> Tone {
    let pace = Pace {
        severity,
        even_pace: None,
        projected: None,
        runs_out_at: runs_out_in.map(|d| now() + d),
    };
    tone(&session(used, 150), &pace, now())
}

#[test]
fn tone_table_covers_every_branch() {
    let mins = |m: i64| Some(SignedDuration::from_mins(m));
    let cases = [
        (Severity::Spent, 100.0, None, Tone::Critical),
        (Severity::Spent, 99.6, mins(10), Tone::Critical),
        (Severity::RunningOut, 60.0, mins(60), Tone::Critical),
        (Severity::RunningOut, 60.0, mins(61), Tone::Warning),
        (Severity::RunningOut, 20.0, mins(5), Tone::Critical),
        (Severity::RunningOut, 20.0, None, Tone::Warning),
        (Severity::RunningOut, 90.0, None, Tone::Critical),
        (Severity::RunningOut, 89.9, mins(120), Tone::Warning),
        (Severity::Close, 10.0, None, Tone::Warning),
        (Severity::Close, 95.0, None, Tone::Warning),
        (Severity::Healthy, 10.0, None, Tone::Good),
        (Severity::Healthy, 89.0, None, Tone::Good),
        (Severity::Untracked, 95.0, None, Tone::Critical),
        (Severity::Untracked, 90.0, None, Tone::Critical),
        (Severity::Untracked, 89.9, None, Tone::Warning),
        (Severity::Untracked, 80.0, None, Tone::Warning),
        (Severity::Untracked, 79.9, None, Tone::Good),
        (Severity::Untracked, 0.0, None, Tone::Good),
    ];
    for (severity, used, runs_out_in, expected) in cases {
        assert_eq!(
            tone_for(severity, used, runs_out_in),
            expected,
            "{severity:?} used {used} runs out in {runs_out_in:?}"
        );
    }
}

#[test]
fn imminence_scales_with_long_periods() {
    let weekly = window(40.0, SignedDuration::from_hours(48), WEEK);
    let pace_at = |hours: i64| Pace {
        severity: Severity::RunningOut,
        even_pace: None,
        projected: None,
        runs_out_at: Some(now() + SignedDuration::from_hours(hours)),
    };
    assert_eq!(tone(&weekly, &pace_at(25), now()), Tone::Critical);
    assert_eq!(tone(&weekly, &pace_at(26), now()), Tone::Warning);
}

#[test]
fn imminent_session_run_out_is_critical() {
    let busy = window(77.0, SignedDuration::from_mins(150), FIVE_HOURS);
    let result = pace(&busy, now());
    assert_eq!(result.severity, Severity::RunningOut);
    let left = result.runs_out_at.unwrap().duration_since(now());
    assert_eq!(left.as_secs() / 60, 44);
    assert_eq!(tone(&busy, &result, now()), Tone::Critical);
}

#[test]
fn distant_run_out_is_a_warning() {
    let weekly = window(40.0, SignedDuration::from_hours(48), WEEK);
    let result = pace(&weekly, now());
    assert_eq!(result.severity, Severity::RunningOut);
    assert_eq!(tone(&weekly, &result, now()), Tone::Warning);
}

#[test]
fn tone_without_data_is_neutral() {
    assert_eq!(Tone::default(), Tone::Neutral);
}

#[test]
fn tone_follows_pace_end_to_end() {
    let running = session(75.0, 150);
    assert_eq!(
        tone(&running, &pace(&running, now()), now()),
        Tone::Critical
    );
    let later = session(60.0, 150);
    assert_eq!(tone(&later, &pace(&later, now()), now()), Tone::Warning);
    let healthy = session(40.0, 150);
    assert_eq!(tone(&healthy, &pace(&healthy, now()), now()), Tone::Good);
    let young_heavy = window(85.0, SignedDuration::from_secs(10), FIVE_HOURS);
    assert_eq!(
        tone(&young_heavy, &pace(&young_heavy, now()), now()),
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
