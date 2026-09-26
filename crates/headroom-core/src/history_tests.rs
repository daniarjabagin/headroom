use super::*;
use crate::account::AccountIdentity;
use crate::quota::WindowId;

fn ts(text: &str) -> Timestamp {
    text.parse().unwrap()
}

fn session(used: f64) -> QuotaWindow {
    QuotaWindow {
        id: WindowId::Session,
        label: "Session".into(),
        used: Percent::new(used),
        resets_at: Some(ts("2026-09-23T12:00:00Z")),
        period: Some(SignedDuration::from_hours(5)),
    }
}

fn sample(at: &str, used: f64) -> UsageSample {
    UsageSample {
        at: ts(at),
        used: Percent::new(used),
    }
}

#[test]
fn sample_step_table() {
    let last = sample("2026-09-23T09:00:00Z", 40.0);
    let later = ts("2026-09-23T09:05:00Z");
    let cases = [
        (None, 40.0, later, Some(SampleStep::Append)),
        (Some(last), 40.0, later, None),
        (Some(last), 41.0, later, Some(SampleStep::Append)),
        (Some(last), 12.0, later, Some(SampleStep::Restart)),
        (Some(last), 41.0, ts("2026-09-23T09:00:00Z"), None),
        (Some(last), 41.0, ts("2026-09-23T08:00:00Z"), None),
        (Some(last), 12.0, ts("2026-09-23T08:00:00Z"), None),
    ];
    for (last, used, at, expected) in cases {
        assert_eq!(
            sample_step(last.as_ref(), &session(used), at),
            expected,
            "{last:?} {used} {at}"
        );
    }
}

#[test]
fn a_sample_from_an_earlier_window_restarts_history() {
    let old = sample("2026-09-23T06:59:59Z", 10.0);
    let step = sample_step(Some(&old), &session(30.0), ts("2026-09-23T07:10:00Z"));
    assert_eq!(step, Some(SampleStep::Restart));
    let behind = sample_step(Some(&old), &session(30.0), ts("2026-09-23T06:00:00Z"));
    assert_eq!(behind, Some(SampleStep::Restart));
}

#[test]
fn current_samples_drop_earlier_windows_and_resets() {
    let samples = [
        sample("2026-09-23T06:00:00Z", 70.0),
        sample("2026-09-23T07:30:00Z", 80.0),
        sample("2026-09-23T08:00:00Z", 5.0),
        sample("2026-09-23T08:10:00Z", 7.0),
    ];
    assert_eq!(current_samples(&samples, &session(8.0)), &samples[2..]);
    assert_eq!(
        current_samples(&samples, &session(3.0)),
        &[] as &[UsageSample]
    );
    assert_eq!(current_samples(&[], &session(3.0)), &[] as &[UsageSample]);
}

#[test]
fn current_samples_without_timing_keep_the_rising_tail() {
    let mut window = session(9.0);
    window.period = None;
    let samples = [
        sample("2026-09-23T06:00:00Z", 70.0),
        sample("2026-09-23T08:00:00Z", 5.0),
    ];
    assert_eq!(current_samples(&samples, &window), &samples[1..]);
}

#[test]
fn retention_keeps_eight_days() {
    let now = ts("2026-09-23T10:00:00Z");
    assert!(is_retained(&sample("2026-09-15T10:00:00Z", 1.0), now));
    assert!(!is_retained(&sample("2026-09-15T09:59:59Z", 1.0), now));
}

#[test]
fn observation_time_follows_the_source() {
    let mut snapshot = LimitsSnapshot {
        identity: AccountIdentity {
            email: None,
            plan: None,
            stable_key: "k".into(),
        },
        windows: Vec::new(),
        balances: Vec::new(),
        notices: Vec::new(),
        fetched_at: ts("2026-09-23T10:00:00Z"),
        source: LimitsSource::Live,
    };
    assert_eq!(observed_at(&snapshot), ts("2026-09-23T10:00:00Z"));
    snapshot.source = LimitsSource::LocalLog {
        observed_at: ts("2026-09-23T08:00:00Z"),
    };
    assert_eq!(observed_at(&snapshot), ts("2026-09-23T08:00:00Z"));
}
