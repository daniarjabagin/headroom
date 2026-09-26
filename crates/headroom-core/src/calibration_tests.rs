use jiff::SignedDuration;

use super::*;

fn now() -> Timestamp {
    "2026-09-23T10:00:00Z".parse().unwrap()
}

fn ago(mins: i64) -> Timestamp {
    now() - SignedDuration::from_mins(mins)
}

fn sample(mins: i64, used: f64) -> UsageSample {
    UsageSample {
        at: ago(mins),
        used: Percent::new(used),
    }
}

fn spent(mins: i64, micros: i64) -> SpendPoint {
    SpendPoint {
        at: ago(mins),
        cost: Some(MicroUsd(micros)),
    }
}

fn close_to(actual: f64, expected: f64) -> bool {
    (actual - expected).abs() < 1e-9
}

fn steady_steps() -> Vec<UsageSample> {
    vec![
        sample(40, 40.0),
        sample(30, 41.0),
        sample(20, 42.0),
        sample(10, 43.0),
    ]
}

fn steady_spend() -> Vec<SpendPoint> {
    vec![spent(35, 100_000), spent(25, 100_000), spent(15, 100_000)]
}

#[test]
fn spend_between_excludes_the_start_and_includes_the_end() {
    let spend = [spent(30, 1), spent(20, 10), spent(10, 100), spent(0, 1_000)];
    let cases = [
        (30, 20, 10),
        (31, 20, 11),
        (30, 0, 1_110),
        (20, 20, 0),
        (10, 30, 0),
        (60, 40, 0),
    ];
    for (from, to, expected) in cases {
        assert_eq!(
            spend_between(&spend, ago(from), ago(to)),
            MicroUsd(expected),
            "{from}..{to}"
        );
    }
}

type Case = (Vec<UsageSample>, Vec<SpendPoint>, Option<(f64, i64)>);

#[test]
fn calibration_needs_enough_rise_and_spend() {
    let cases: [Case; 6] = [
        (steady_steps(), steady_spend(), Some((3.0, 300_000))),
        (steady_steps()[1..].to_vec(), steady_spend(), None),
        (
            steady_steps(),
            vec![spent(35, 10_000), spent(25, 10_000), spent(15, 10_000)],
            None,
        ),
        (steady_steps(), Vec::new(), None),
        (vec![sample(10, 43.0)], steady_spend(), None),
        (
            vec![sample(40, 40.0), sample(10, 46.0)],
            vec![spent(50, 900_000), spent(20, 120_000)],
            Some((6.0, 120_000)),
        ),
    ];
    for (samples, spend, expected) in cases {
        let calibration = calibrate(&samples, &spend, ago(10));
        let expected = expected.map(|(rise, micros)| Calibration {
            rise,
            spend: MicroUsd(micros),
        });
        assert_eq!(calibration, expected, "{samples:?}");
    }
}

#[test]
fn calibration_uses_the_latest_pairs_up_to_enough_rise() {
    let samples = [
        sample(90, 10.0),
        sample(60, 20.0),
        sample(30, 30.0),
        sample(10, 50.0),
    ];
    let spend = [
        spent(70, 5_000_000),
        spent(40, 900_000),
        spent(20, 2_000_000),
    ];
    let calibration = calibrate(&samples, &spend, ago(10)).unwrap();
    assert_eq!(
        calibration,
        Calibration {
            rise: 20.0,
            spend: MicroUsd(2_000_000),
        }
    );
    assert!(close_to(calibration.percent_of(MicroUsd(100_000)), 1.0));
}

fn calibrated() -> Calibration {
    calibrate(&steady_steps(), &steady_spend(), ago(10)).unwrap()
}

fn observed(used: f64, changed: i64, seen: i64) -> Observed {
    Observed {
        used: Percent::new(used),
        changed_at: ago(changed),
        observed_at: ago(seen),
    }
}

#[test]
fn the_estimate_adds_calibrated_spend_since_the_last_change() {
    let cases = [
        (observed(43.0, 10, 10), vec![], 43.0),
        (observed(43.0, 10, 10), vec![spent(5, 50_000)], 43.5),
        (observed(43.0, 10, 10), vec![spent(12, 900_000)], 43.0),
        (
            observed(43.0, 10, 2),
            vec![spent(8, 200_000), spent(1, 50_000)],
            44.5,
        ),
        (
            observed(43.0, 10, 10),
            vec![spent(5, 400_000), spent(1, 400_000)],
            51.0,
        ),
        (observed(98.0, 10, 10), vec![spent(1, 900_000)], 100.0),
        (observed(43.0, 10, 30), vec![spent(5, 50_000)], 43.5),
    ];
    for (observation, spend, expected) in cases {
        let estimate = calibrated().estimate_used(observation, &spend, now());
        assert!(
            close_to(estimate.value(), expected),
            "{observation:?} {spend:?}: {estimate:?}"
        );
    }
}

#[test]
fn the_rate_is_calibrated_spend_per_second() {
    let calibration = calibrated();
    let spend = [spent(20, 300_000), spent(5, 300_000)];
    let rate = calibration.rate(&spend, ago(30), now()).unwrap();
    assert!(close_to(rate, 6.0 / 1_800.0));
    assert_eq!(calibration.rate(&[], ago(30), now()), None);
    assert_eq!(calibration.rate(&spend, now(), now()), None);
}

fn unpriced(mins: i64) -> SpendPoint {
    SpendPoint {
        at: ago(mins),
        cost: None,
    }
}

#[test]
fn spend_since_the_last_step_without_a_step_dilutes_the_ratio() {
    let mut spend = steady_spend();
    spend.push(spent(5, 300_000));
    let calibration = calibrate(&steady_steps(), &spend, ago(2)).unwrap();
    assert_eq!(
        calibration,
        Calibration {
            rise: 3.0,
            spend: MicroUsd(600_000),
        }
    );
    let before_the_poll = calibrate(&steady_steps(), &spend, ago(10)).unwrap();
    assert!(close_to(before_the_poll.percent_of(MicroUsd(100_000)), 1.0));
}

#[test]
fn unpriced_usage_inside_the_calibration_leaves_it_out() {
    let with = |point: SpendPoint| {
        let mut spend = steady_spend();
        spend.push(point);
        spend.sort_unstable();
        calibrate(&steady_steps(), &spend, ago(2))
    };
    assert_eq!(with(unpriced(25)), None);
    assert_eq!(with(unpriced(5)), None);
    assert!(with(unpriced(45)).is_some());
    assert!(with(unpriced(1)).is_some());
}

#[test]
fn unpriced_points_weigh_nothing_but_are_found() {
    let spend = [spent(20, 100), unpriced(15), spent(10, 1_000)];
    assert_eq!(spend_between(&spend, ago(30), now()), MicroUsd(1_100));
    assert!(has_unpriced(&spend, ago(30), now()));
    assert!(!has_unpriced(&spend, ago(15), now()));
    assert!(!has_unpriced(&spend, ago(30), ago(20)));
}

#[test]
fn spend_the_provider_should_have_shown_contradicts_the_calibration() {
    let calibration = calibrated();
    let cases = [
        (observed(43.0, 10, 2), vec![spent(5, 100_000)], false),
        (observed(43.0, 10, 2), vec![spent(5, 100_001)], true),
        (observed(43.0, 10, 2), vec![spent(1, 900_000)], false),
        (observed(43.0, 10, 10), vec![spent(5, 900_000)], false),
    ];
    for (observation, spend, expected) in cases {
        assert_eq!(
            calibration.contradicts(observation, &spend),
            expected,
            "{observation:?} {spend:?}"
        );
    }
}
