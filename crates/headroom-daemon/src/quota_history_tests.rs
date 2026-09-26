use headroom_core::units::Percent;

use super::*;
use crate::testing::{CODEX, account, session, snapshot, ts, weekly};

const RESET: &str = "2026-09-23T12:00:00Z";
const NEXT_RESET: &str = "2026-09-23T17:00:00Z";

fn id() -> AccountId {
    account(CODEX, "work").id
}

fn observe(history: &mut QuotaHistory, used: f64, reset: &str, at: &str) {
    let limits = snapshot(vec![session(used, reset)], at);
    history.record(&id(), &limits, ts(at));
}

fn used_values(history: &QuotaHistory) -> Vec<f64> {
    window_samples(history.account(&id()), "session")
        .iter()
        .map(|sample| sample.used.value())
        .collect()
}

#[test]
fn only_changes_are_recorded() {
    let mut history = QuotaHistory::default();
    observe(&mut history, 40.0, RESET, "2026-09-23T09:00:00Z");
    observe(&mut history, 40.0, RESET, "2026-09-23T09:05:00Z");
    observe(&mut history, 41.0, RESET, "2026-09-23T09:10:00Z");
    observe(&mut history, 41.0, RESET, "2026-09-23T09:15:00Z");
    assert_eq!(used_values(&history), vec![40.0, 41.0]);
    let samples = window_samples(history.account(&id()), "session");
    assert_eq!(samples[1].at, ts("2026-09-23T09:10:00Z"));
}

#[test]
fn a_window_reset_clears_its_history() {
    let mut history = QuotaHistory::default();
    observe(&mut history, 70.0, RESET, "2026-09-23T11:00:00Z");
    observe(&mut history, 80.0, RESET, "2026-09-23T11:30:00Z");
    observe(&mut history, 2.0, NEXT_RESET, "2026-09-23T12:10:00Z");
    assert_eq!(used_values(&history), vec![2.0]);
}

#[test]
fn samples_older_than_eight_days_are_pruned() {
    let mut history = QuotaHistory::default();
    let first = snapshot(
        vec![weekly(10.0, "2026-09-30T00:00:00Z")],
        "2026-09-15T09:00:00Z",
    );
    history.record(&id(), &first, ts("2026-09-15T09:00:00Z"));
    observe(&mut history, 5.0, RESET, "2026-09-23T09:30:00Z");
    assert!(window_samples(history.account(&id()), "weekly").is_empty());
    assert_eq!(used_values(&history), vec![5.0]);
}

#[test]
fn rows_group_by_account_and_window() {
    let sample = |at: &str, used: f64| UsageSample {
        at: ts(at),
        used: Percent::new(used),
    };
    let rows = vec![
        (
            id(),
            "session".to_owned(),
            sample("2026-09-23T09:00:00Z", 1.0),
        ),
        (
            id(),
            "session".to_owned(),
            sample("2026-09-23T09:05:00Z", 2.0),
        ),
        (
            id(),
            "weekly".to_owned(),
            sample("2026-09-23T09:00:00Z", 3.0),
        ),
    ];
    let history = QuotaHistory::from_rows(rows);
    assert_eq!(used_values(&history), vec![1.0, 2.0]);
    assert_eq!(window_samples(history.account(&id()), "weekly").len(), 1);
    assert!(window_samples(None, "session").is_empty());
}
