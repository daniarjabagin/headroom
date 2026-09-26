use super::*;
use crate::quota_history::QuotaHistory;
use crate::storage::Storage;
use crate::testing::{CODEX, account, session, snapshot, ts, weekly};

const RESET: &str = "2026-09-23T12:00:00Z";
const NEXT_RESET: &str = "2026-09-23T17:00:00Z";

fn observed(session_used: f64, reset: &str, at: &str) -> (LimitsSnapshot, Timestamp) {
    let windows = vec![
        session(session_used, reset),
        weekly(20.0, "2026-09-26T10:00:00Z"),
    ];
    (snapshot(windows, at), ts(at))
}

fn steps() -> Vec<(LimitsSnapshot, Timestamp)> {
    vec![
        observed(40.0, RESET, "2026-09-23T09:00:00Z"),
        observed(40.0, RESET, "2026-09-23T09:05:00Z"),
        observed(45.0, RESET, "2026-09-23T09:10:00Z"),
        observed(90.0, RESET, "2026-09-23T11:50:00Z"),
        observed(3.0, NEXT_RESET, "2026-09-23T12:05:00Z"),
        observed(4.0, NEXT_RESET, "2026-09-23T12:10:00Z"),
    ]
}

fn stored(storage: &Storage) -> Vec<(String, f64)> {
    storage
        .blocking(|conn| load_all(conn))
        .unwrap()
        .into_iter()
        .map(|(_, window, sample)| (window, sample.used.value()))
        .collect()
}

#[test]
fn records_changes_and_restarts_after_a_reset() {
    let storage = Storage::open_in_memory().unwrap();
    let id = account(CODEX, "work").id;
    for (limits, now) in steps() {
        storage
            .blocking(|conn| record(conn, &id, &limits, now))
            .unwrap();
    }
    assert_eq!(
        stored(&storage),
        vec![
            ("session".to_owned(), 3.0),
            ("session".to_owned(), 4.0),
            ("weekly".to_owned(), 20.0),
        ]
    );
}

#[test]
fn stored_history_matches_the_in_memory_history() {
    let storage = Storage::open_in_memory().unwrap();
    let id = account(CODEX, "work").id;
    let mut memory = QuotaHistory::default();
    for (limits, now) in steps() {
        storage
            .blocking(|conn| record(conn, &id, &limits, now))
            .unwrap();
        memory.record(&id, &limits, now);
    }
    let loaded = storage.blocking(|conn| load_all(conn)).unwrap();
    assert_eq!(QuotaHistory::from_rows(loaded), memory);
}

#[test]
fn samples_older_than_eight_days_are_pruned() {
    let storage = Storage::open_in_memory().unwrap();
    let id = account(CODEX, "work").id;
    let old = snapshot(
        vec![weekly(10.0, "2026-09-20T00:00:00Z")],
        "2026-09-15T09:00:00Z",
    );
    let fresh = snapshot(vec![session(5.0, RESET)], "2026-09-23T09:30:00Z");
    storage
        .blocking(|conn| {
            record(conn, &id, &old, ts("2026-09-15T09:00:00Z"))?;
            record(conn, &id, &fresh, ts("2026-09-23T09:30:00Z"))
        })
        .unwrap();
    assert_eq!(stored(&storage), vec![("session".to_owned(), 5.0)]);
}

#[test]
fn a_stale_local_observation_is_not_recorded_twice() {
    let storage = Storage::open_in_memory().unwrap();
    let id = account(CODEX, "work").id;
    let later = snapshot(vec![session(50.0, RESET)], "2026-09-23T09:30:00Z");
    let earlier = snapshot(vec![session(48.0, RESET)], "2026-09-23T09:20:00Z");
    storage
        .blocking(|conn| {
            record(conn, &id, &later, ts("2026-09-23T09:30:00Z"))?;
            record(conn, &id, &earlier, ts("2026-09-23T09:31:00Z"))
        })
        .unwrap();
    assert_eq!(stored(&storage), vec![("session".to_owned(), 50.0)]);
}
