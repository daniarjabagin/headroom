use headroom_core::quota::{Balance, BalanceAmount, WindowId};

use super::super::test_support::{at, set_mtime, write_rollout};
use super::*;

const RATE_LIMITS: &str = include_str!("fixtures/rollout_rate_limits.jsonl");
const LEGACY: &str = include_str!("fixtures/rollout_legacy.jsonl");

fn identity() -> AccountIdentity {
    AccountIdentity {
        email: Some("someone@example.com".into()),
        plan: Some("Plus".into()),
        stable_key: "u/a".into(),
    }
}

#[test]
fn newest_main_snapshot_becomes_local_log_limits() {
    let home = tempfile::tempdir().unwrap();
    write_rollout(
        home.path(),
        "sessions/2026/09/22/rollout.jsonl",
        RATE_LIMITS,
    );
    let now = at("2026-09-23T10:00:00Z");
    let snapshot = latest_snapshot(home.path(), identity(), None, now)
        .unwrap()
        .unwrap();
    let observed_at = at("2026-09-22T08:14:30.537Z");
    assert_eq!(snapshot.source, LimitsSource::LocalLog { observed_at });
    assert_eq!(snapshot.fetched_at, observed_at);
    assert_eq!(snapshot.identity.plan.as_deref(), Some("Pro 5x"));
    assert_eq!(snapshot.identity.stable_key, "u/a");
    assert_eq!(snapshot.windows.len(), 1);
    let weekly = &snapshot.windows[0];
    assert_eq!(weekly.id, WindowId::Weekly);
    assert_eq!(weekly.used, Percent::new(64.0));
    assert_eq!(weekly.resets_at, Some(at("2026-09-26T18:49:14Z")));
    assert_eq!(weekly.period, Some(WindowId::WEEKLY_PERIOD));
    assert_eq!(
        snapshot.balances,
        [Balance {
            id: "credits".into(),
            label: "Credits".into(),
            amount: BalanceAmount::Count {
                value: 12,
                unit: "credits".into()
            },
        }]
    );
}

#[test]
fn zero_credit_balance_is_not_shown_offline() {
    let home = tempfile::tempdir().unwrap();
    let first_line = RATE_LIMITS.lines().next().unwrap();
    write_rollout(home.path(), "sessions/rollout.jsonl", first_line);
    let snapshot = latest_snapshot(home.path(), identity(), None, at("2026-09-23T10:00:00Z"))
        .unwrap()
        .unwrap();
    assert_eq!(snapshot.windows.len(), 1);
    assert!(snapshot.balances.is_empty());
}

#[test]
fn window_that_already_reset_is_shown_empty() {
    let home = tempfile::tempdir().unwrap();
    write_rollout(home.path(), "sessions/rollout.jsonl", RATE_LIMITS);
    let after_reset = at("2026-09-28T00:00:00Z");
    let snapshot = latest_snapshot(home.path(), identity(), None, after_reset)
        .unwrap()
        .unwrap();
    assert_eq!(snapshot.windows[0].used, Percent::ZERO);
    assert_eq!(snapshot.windows[0].resets_at, None);
    assert_eq!(snapshot.windows[0].period, Some(WindowId::WEEKLY_PERIOD));
}

#[test]
fn legacy_snapshot_without_reset_time_still_classifies_windows() {
    let home = tempfile::tempdir().unwrap();
    write_rollout(home.path(), "sessions/rollout.jsonl", LEGACY);
    let snapshot = latest_snapshot(home.path(), identity(), None, at("2026-07-10T09:00:00Z"))
        .unwrap()
        .unwrap();
    let ids: Vec<_> = snapshot
        .windows
        .iter()
        .map(|window| window.id.clone())
        .collect();
    assert_eq!(ids, [WindowId::Session, WindowId::Weekly]);
    assert_eq!(snapshot.identity.plan.as_deref(), Some("Plus"));
}

#[test]
fn picks_newest_observation_across_files() {
    let home = tempfile::tempdir().unwrap();
    let older = write_rollout(home.path(), "sessions/a.jsonl", RATE_LIMITS);
    let newer_line = RATE_LIMITS
        .lines()
        .nth(1)
        .unwrap()
        .replace("2026-09-22T08:14:30.537Z", "2026-09-22T09:00:00.000Z")
        .replace("64.0", "70.0");
    let newer = write_rollout(
        home.path(),
        "archived_sessions/b.jsonl",
        &format!("{newer_line}\n"),
    );
    set_mtime(&older, "2026-09-22T08:42:00Z");
    set_mtime(&newer, "2026-09-22T09:00:01Z");
    let found = latest_observation(home.path(), None).unwrap().unwrap();
    assert_eq!(found.observed_at, at("2026-09-22T09:00:00Z"));
}

#[test]
fn stale_mtime_order_still_finds_the_newest_line() {
    let home = tempfile::tempdir().unwrap();
    let busy = write_rollout(home.path(), "sessions/busy.jsonl", RATE_LIMITS);
    let quiet_line = RATE_LIMITS
        .lines()
        .nth(1)
        .unwrap()
        .replace("2026-09-22T08:14:30.537Z", "2026-09-22T08:30:00.000Z");
    let quiet = write_rollout(
        home.path(),
        "sessions/quiet.jsonl",
        &format!("{quiet_line}\n"),
    );
    set_mtime(&busy, "2026-09-22T08:42:00Z");
    set_mtime(&quiet, "2026-09-22T08:30:01Z");
    let found = latest_observation(home.path(), None).unwrap().unwrap();
    assert_eq!(found.observed_at, at("2026-09-22T08:30:00Z"));
}

#[test]
fn no_logs_means_no_snapshot() {
    let home = tempfile::tempdir().unwrap();
    assert_eq!(
        latest_snapshot(home.path(), identity(), None, at("2026-09-23T10:00:00Z")).unwrap(),
        None
    );
    write_rollout(
        home.path(),
        "sessions/empty.jsonl",
        "{\"type\":\"session_meta\"}\n",
    );
    assert_eq!(latest_observation(home.path(), None).unwrap(), None);
}

#[test]
fn observations_before_the_login_are_ignored() {
    let home = tempfile::tempdir().unwrap();
    let older = write_rollout(home.path(), "sessions/a.jsonl", RATE_LIMITS);
    set_mtime(&older, "2026-09-22T08:42:00Z");
    let login = Some(at("2026-09-22T08:15:00Z"));
    assert_eq!(latest_observation(home.path(), login).unwrap(), None);
    let earlier_login = Some(at("2026-09-22T08:00:00Z"));
    let found = latest_observation(home.path(), earlier_login)
        .unwrap()
        .unwrap();
    assert_eq!(found.observed_at, at("2026-09-22T08:14:30.537Z"));
}

#[test]
fn newest_observation_after_the_login_wins_over_older_files() {
    let home = tempfile::tempdir().unwrap();
    let older = write_rollout(home.path(), "sessions/a.jsonl", RATE_LIMITS);
    let newer_line = RATE_LIMITS
        .lines()
        .nth(1)
        .unwrap()
        .replace("2026-09-22T08:14:30.537Z", "2026-09-22T09:00:00.000Z");
    let newer = write_rollout(home.path(), "sessions/b.jsonl", &format!("{newer_line}\n"));
    set_mtime(&older, "2026-09-22T08:42:00Z");
    set_mtime(&newer, "2026-09-22T09:00:01Z");
    let login = Some(at("2026-09-22T08:50:00Z"));
    let snapshot = latest_snapshot(home.path(), identity(), login, at("2026-09-23T10:00:00Z"))
        .unwrap()
        .unwrap();
    assert_eq!(snapshot.fetched_at, at("2026-09-22T09:00:00Z"));
}
