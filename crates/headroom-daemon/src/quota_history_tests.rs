use std::path::PathBuf;

use headroom_core::account::CredentialOwner;
use headroom_core::units::{MicroUsd, Percent};
use jiff::SignedDuration;

use super::*;
use crate::storage::accounts::AccountRecord;
use crate::testing::{CODEX, account, session, snapshot, ts, usage_home_of, weekly};

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

fn spent(at: &str, micros: i64) -> SpendPoint {
    SpendPoint {
        at: ts(at),
        cost: Some(MicroUsd(micros)),
    }
}

fn headroom_owned() -> AccountRef {
    AccountRef {
        home: PathBuf::from("/home/ada/.local/share/headroom/codex-work"),
        owner: CredentialOwner::Headroom,
        ..account(CODEX, "work")
    }
}

fn model_with(reference: &AccountRef) -> Model {
    let record = AccountRecord {
        reference: reference.clone(),
        label: None,
        hidden: false,
        sort_order: 0,
        email: None,
        plan: None,
        last_seen: ts("2026-09-23T10:00:00Z"),
        gone: false,
    };
    Model {
        accounts: vec![record],
        ..Model::default()
    }
}

#[test]
fn a_linked_home_counts_whole_including_spend_from_before_the_switch() {
    let own = headroom_owned();
    let cli = account(CODEX, "work");
    let mut model = model_with(&own);
    model.cli_sign_ins.set(&CODEX, std::slice::from_ref(&cli));
    let before_switch = spent("2026-09-23T08:00:00Z", 5_000_000);
    let after_switch = spent("2026-09-23T09:30:00Z", 200);
    model
        .history
        .set_spend(&usage_home_of(&cli), vec![before_switch, after_switch]);
    let mine = spent("2026-09-23T09:00:00Z", 100);
    model.history.set_spend(&usage_home_of(&own), vec![mine]);
    let merged = model.account_spend(&own);
    assert_eq!(merged.as_ref(), [before_switch, mine, after_switch]);
}

#[test]
fn live_spend_is_only_gathered_for_a_live_account_with_a_short_window() {
    let work = account(CODEX, "work");
    let mut model = model_with(&work);
    let home = usage_home_of(&work);
    model
        .history
        .set_spend(&home, vec![spent("2026-09-23T09:00:00Z", 100)]);
    let live = Signal {
        liveness: Liveness::Live,
        poll_interval: SignedDuration::from_mins(5),
    };
    let idle = Signal {
        liveness: Liveness::Idle,
        ..live
    };
    let short = snapshot(vec![session(10.0, RESET)], "2026-09-23T09:30:00Z");
    let long = snapshot(
        vec![weekly(10.0, "2026-09-30T00:00:00Z")],
        "2026-09-23T09:30:00Z",
    );
    let shared = model.history.spend_of(std::slice::from_ref(&home));
    assert!(Arc::ptr_eq(&model.live_spend(&work, &short, live), &shared));
    assert!(model.live_spend(&work, &short, idle).is_empty());
    assert!(model.live_spend(&work, &long, live).is_empty());
}

#[test]
fn spend_of_homes_no_longer_in_use_is_dropped() {
    let work = account(CODEX, "work");
    let mut model = model_with(&work);
    let home = usage_home_of(&work);
    let gone = UsageHome {
        provider: CODEX,
        home: PathBuf::from("/home/ada/.codex-old"),
    };
    model.usage_homes.insert(home.clone());
    model
        .history
        .set_spend(&gone, vec![spent("2026-09-23T09:00:00Z", 1)]);
    model.store_spend(&home, vec![spent("2026-09-23T09:10:00Z", 2)]);
    assert!(model.history.spend_of(&[gone]).is_empty());
    assert_eq!(model.history.spend_of(&[home]).len(), 1);
}
