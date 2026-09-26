use headroom_core::calibration::SpendPoint;
use headroom_core::history::UsageSample;
use headroom_core::pace::{Basis, Severity};
use headroom_core::units::{MicroUsd, Percent};
use jiff::SignedDuration;

use crate::model::Model;
use crate::quota_history::QuotaHistory;
use crate::state::payload::WindowView;
use crate::state::tests::{assemble_sample, sample_model};
use crate::testing::{ts, usage_home_of};

const NOW: &str = "2026-09-23T10:00:00Z";

fn session_rows(model: &Model) -> QuotaHistory {
    let work = model.accounts[0].id().clone();
    let rows = [
        ("2026-09-23T09:49:00Z", 52.0),
        ("2026-09-23T09:52:00Z", 53.0),
        ("2026-09-23T09:55:00Z", 54.0),
        ("2026-09-23T09:58:00Z", 55.0),
    ];
    QuotaHistory::from_rows(
        rows.iter()
            .map(|(at, used)| {
                let sample = UsageSample {
                    at: ts(at),
                    used: Percent::new(*used),
                };
                (work.clone(), "session".to_owned(), sample)
            })
            .collect(),
    )
}

fn steady_spend() -> Vec<SpendPoint> {
    (1..=15)
        .rev()
        .map(|mins| SpendPoint {
            at: ts(NOW) - SignedDuration::from_mins(mins),
            cost: MicroUsd(20_000),
        })
        .collect()
}

fn burst_spend() -> Vec<SpendPoint> {
    let mut spend = steady_spend();
    spend.push(SpendPoint {
        at: ts(NOW) - SignedDuration::from_secs(30),
        cost: MicroUsd(1_500_000),
    });
    spend
}

fn model_with(spend: Vec<SpendPoint>, live: bool) -> Model {
    let mut model = sample_model();
    model.history = session_rows(&model);
    let home = usage_home_of(&model.accounts[0].reference);
    model.history.set_spend(&home, spend);
    if live {
        model.activity.record(&home, ts(NOW), ts(NOW));
    }
    model
}

fn windows(model: &Model) -> Vec<WindowView> {
    assemble_sample(model)
        .accounts
        .into_iter()
        .flat_map(|account| account.windows)
        .collect()
}

#[test]
fn a_live_burst_between_polls_moves_the_run_out_forward() {
    let steady = windows(&model_with(steady_spend(), true));
    let burst = windows(&model_with(burst_spend(), true));
    assert_eq!(steady[0].pace.basis, Some(Basis::Recent));
    assert_eq!(steady[0].pace.severity, Severity::Healthy);
    assert_eq!(steady[0].pace.runs_out_at, None);
    assert_eq!(burst[0].pace.basis, Some(Basis::Recent));
    assert_eq!(burst[0].pace.severity, Severity::RunningOut);
    let runs_out_at = burst[0].pace.runs_out_at.unwrap();
    assert!(runs_out_at < ts("2026-09-23T10:40:00Z"), "{runs_out_at}");
    for view in [&steady[0], &burst[0]] {
        assert!((view.used_percent - 55.0).abs() < f64::EPSILON);
        assert!((view.remaining_percent - 45.0).abs() < f64::EPSILON);
    }
}

#[test]
fn spend_changes_nothing_for_an_account_that_is_not_live() {
    let without = windows(&model_with(Vec::new(), false));
    let with = windows(&model_with(burst_spend(), false));
    assert_eq!(with, without);
}

#[test]
fn spend_only_touches_the_live_short_window() {
    let without = windows(&model_with(Vec::new(), true));
    let with = windows(&model_with(burst_spend(), true));
    assert_ne!(with[0], without[0]);
    assert_eq!(with[1..], without[1..]);
}
