use super::*;
use crate::quota::WindowId;
use crate::units::MicroUsd;

const FIVE_HOURS: SignedDuration = SignedDuration::from_hours(5);
const WEEK: SignedDuration = SignedDuration::from_hours(7 * 24);
const POLL: SignedDuration = SignedDuration::from_mins(5);

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

fn polls() -> Vec<UsageSample> {
    vec![
        ago(40, 40.0),
        ago(30, 41.0),
        ago(20, 42.0),
        ago(10, 43.0),
        ago(5, 44.0),
    ]
}

fn steady_spend() -> Vec<SpendPoint> {
    (1..=45)
        .rev()
        .map(|mins| SpendPoint {
            at: now() - SignedDuration::from_mins(mins),
            cost: MicroUsd(10_000),
        })
        .collect()
}

fn with_burst() -> Vec<SpendPoint> {
    let mut spend = steady_spend();
    spend.push(SpendPoint {
        at: now() - SignedDuration::from_secs(30),
        cost: MicroUsd(3_000_000),
    });
    spend
}

fn run(
    window: &QuotaWindow,
    samples: &[UsageSample],
    liveness: Liveness,
    spend: &[SpendPoint],
) -> Pace {
    let activity = Activity {
        samples,
        signal: Signal {
            liveness,
            poll_interval: POLL,
        },
        observed_at: now() - SignedDuration::from_mins(1),
    };
    forecast_with_spend(window, activity, spend, now())
}

#[test]
fn a_burst_between_polls_brings_the_run_out_forward() {
    let busy = session(44.0);
    let by_percent = run(&busy, &polls(), Liveness::Live, &[]);
    let steady = run(&busy, &polls(), Liveness::Live, &steady_spend());
    let burst = run(&busy, &polls(), Liveness::Live, &with_burst());
    assert_eq!(by_percent.basis, Some(Basis::Recent));
    assert_eq!(by_percent.runs_out_at, None);
    assert_eq!(steady.basis, Some(Basis::Recent));
    assert_eq!(steady.severity, Severity::Healthy);
    assert_eq!(steady.runs_out_at, None);
    assert_eq!(burst.basis, Some(Basis::Recent));
    assert_eq!(burst.severity, Severity::RunningOut);
    let runs_out_at = burst.runs_out_at.unwrap();
    assert!(
        runs_out_at < now() + SignedDuration::from_mins(30),
        "{runs_out_at}"
    );
    assert!(burst.projected > steady.projected);
    assert_eq!(burst.even_pace, pace(&busy, now()).even_pace);
}

#[test]
fn steady_spend_projects_from_the_calibrated_rate() {
    let busy = session(44.0);
    let steady = run(&busy, &polls(), Liveness::Live, &steady_spend());
    let per_micro = 4.0 / 350_000.0;
    let estimated = 44.0 + 40_000.0 * per_micro;
    let rate = 450_000.0 * per_micro / 3_000.0;
    let expected = estimated + rate * 9_000.0;
    let projected = f64::from(steady.projected.unwrap());
    assert!(
        (projected - expected).abs() < 1e-9,
        "{projected} {expected}"
    );
}

#[test]
fn only_live_calibrated_short_windows_use_spend() {
    let fresh = window(12.0, SignedDuration::from_mins(60), FIVE_HOURS);
    let reset = vec![
        ago(80, 70.0),
        ago(75, 80.0),
        ago(70, 90.0),
        ago(3, 11.0),
        ago(0, 12.0),
    ];
    let week = window(44.0, SignedDuration::from_hours(72), WEEK);
    let thin: Vec<_> = steady_spend().into_iter().take(5).collect();
    let cases = [
        (session(44.0), polls(), Liveness::Unknown, with_burst()),
        (session(44.0), polls(), Liveness::Idle, with_burst()),
        (session(44.0), polls(), Liveness::Live, thin),
        (
            session(44.0),
            polls()[2..].to_vec(),
            Liveness::Live,
            with_burst(),
        ),
        (fresh, reset, Liveness::Live, with_burst()),
        (week, polls(), Liveness::Live, with_burst()),
    ];
    for (window, samples, liveness, spend) in cases {
        let without = run(&window, &samples, liveness, &[]);
        let with = run(&window, &samples, liveness, &spend);
        assert_eq!(with, without, "{window:?} {liveness:?}");
    }
}

#[test]
fn an_idle_account_stays_paused_whatever_it_spent_before() {
    let busy = session(60.0);
    let samples = [ago(40, 57.0), ago(35, 58.0), ago(30, 59.0), ago(25, 60.0)];
    let activity = Activity {
        samples: &samples,
        signal: Signal {
            liveness: Liveness::Idle,
            poll_interval: POLL,
        },
        observed_at: now(),
    };
    let result = forecast_with_spend(&busy, activity, &with_burst(), now());
    assert_eq!(result.basis, Some(Basis::Paused));
    assert_eq!(result.runs_out_at, None);
}
