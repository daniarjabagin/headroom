use super::*;
use crate::units::Percent;

fn now() -> Timestamp {
    "2026-09-23T10:00:00Z".parse().unwrap()
}

fn at_mins(mins_ago: i64, used: f64) -> UsageSample {
    UsageSample {
        at: now() - SignedDuration::from_mins(mins_ago),
        used: Percent::new(used),
    }
}

fn session() -> Cadence {
    Cadence::of(SignedDuration::from_hours(5), SignedDuration::from_mins(5))
}

fn close_to(actual: Option<f64>, expected: f64) -> bool {
    actual.is_some_and(|value| (value - expected).abs() < 1e-12)
}

#[test]
fn cadence_scales_with_the_period() {
    let cases = [
        (SignedDuration::from_mins(5), 10, 30),
        (SignedDuration::from_hours(5), 10, 50),
        (SignedDuration::from_hours(24), 48, 240),
        (SignedDuration::from_hours(7 * 24), 120, 240),
        (SignedDuration::from_hours(30 * 24), 120, 240),
    ];
    for (period, idle, lookback) in cases {
        let cadence = Cadence::of(period, SignedDuration::from_mins(5));
        assert_eq!(
            cadence.idle_after,
            SignedDuration::from_mins(idle),
            "{period}"
        );
        assert_eq!(
            cadence.lookback,
            SignedDuration::from_mins(lookback),
            "{period}"
        );
    }
}

#[test]
fn steady_steps_give_their_rate() {
    let samples = [
        at_mins(9, 10.0),
        at_mins(6, 11.0),
        at_mins(3, 12.0),
        at_mins(0, 13.0),
    ];
    assert!(close_to(
        recent_rate(&samples, session(), now()),
        3.0 / 540.0
    ));
    assert!(close_to(last_active_rate(&samples, session()), 3.0 / 540.0));
}

#[test]
fn fewer_than_three_steps_are_not_trusted() {
    let samples = [at_mins(6, 10.0), at_mins(3, 11.0), at_mins(0, 12.0)];
    assert_eq!(recent_rate(&samples, session(), now()), None);
    assert_eq!(last_active_rate(&samples, session()), None);
    assert_eq!(recent_rate(&[], session(), now()), None);
}

#[test]
fn an_idle_gap_ends_the_active_span() {
    let samples = [
        at_mins(200, 1.0),
        at_mins(190, 2.0),
        at_mins(180, 3.0),
        at_mins(170, 4.0),
        at_mins(6, 5.0),
        at_mins(3, 6.0),
        at_mins(0, 7.0),
    ];
    assert_eq!(recent_rate(&samples, session(), now()), None);
    let resumed = [samples.as_slice(), &[at_mins(-3, 8.0)]].concat();
    let later = now() + SignedDuration::from_mins(3);
    assert!(close_to(
        recent_rate(&resumed, session(), later),
        3.0 / 540.0
    ));
}

#[test]
fn the_lookback_keeps_only_recent_active_time() {
    let mut samples: Vec<_> = (0..10)
        .map(|step| at_mins(100 - step * 10, f64::from(u8::try_from(step).unwrap())))
        .collect();
    samples.extend([at_mins(4, 11.0), at_mins(2, 13.0), at_mins(0, 15.0)]);
    let rate = recent_rate(&samples, session(), now()).unwrap();
    let expected = (15.0 - 5.0) / 3_000.0;
    assert!((rate - expected).abs() < 1e-12, "{rate}");
}

#[test]
fn an_overdue_step_slows_the_rate() {
    let samples = [
        at_mins(15, 10.0),
        at_mins(12, 11.0),
        at_mins(9, 12.0),
        at_mins(6, 13.0),
    ];
    assert!(close_to(
        recent_rate(&samples, session(), now()),
        3.0 / 720.0
    ));
    let within_step = now() - SignedDuration::from_mins(3);
    assert!(close_to(
        recent_rate(&samples, session(), within_step),
        3.0 / 540.0
    ));
}

#[test]
fn an_idle_account_keeps_its_last_active_rate() {
    let samples = [
        at_mins(40, 10.0),
        at_mins(35, 20.0),
        at_mins(30, 30.0),
        at_mins(25, 40.0),
    ];
    assert!(session().is_idle(&samples, now()));
    assert_eq!(recent_rate(&samples, session(), now()), None);
    assert!(close_to(
        last_active_rate(&samples, session()),
        30.0 / 900.0
    ));
}

#[test]
fn a_long_poll_interval_stretches_the_idle_threshold() {
    let hourly = Cadence::of(SignedDuration::from_hours(5), SignedDuration::from_hours(1));
    assert_eq!(hourly.idle_after, SignedDuration::from_hours(2));
    let live = Cadence::of(SignedDuration::from_hours(5), SignedDuration::from_mins(1));
    assert_eq!(live.idle_after, SignedDuration::from_mins(10));
}
