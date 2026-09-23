use std::path::PathBuf;

use headroom_core::cursor::LogCursors;
use headroom_core::units::MicroUsd;
use rusqlite::Connection;

use super::*;
use crate::storage::migrations;
use crate::testing::{CODEX, event, ts};

const VERSION_2: [&str; 2] = [
    include_str!("migrations/001_init.sql"),
    include_str!("migrations/002_subscription_lapses.sql"),
];

fn home() -> UsageHome {
    UsageHome {
        provider: CODEX,
        home: PathBuf::from("/home/ada/.grok"),
    }
}

fn migrated() -> Connection {
    let mut conn = Connection::open_in_memory().unwrap();
    migrations::migrate(&mut conn).unwrap();
    conn
}

fn version_2_database() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    for sql in VERSION_2 {
        conn.execute_batch(sql).unwrap();
    }
    conn.pragma_update(None, "user_version", 2).unwrap();
    conn
}

fn since() -> Timestamp {
    ts("2026-09-01T00:00:00Z")
}

fn priced(key: &str, output: u64, cost: i64) -> UsageEvent {
    UsageEvent {
        reported_cost: Some(MicroUsd(cost)),
        ..event(key, "2026-09-23T09:00:00Z", "grok-4.7-build", 100, output)
    }
}

#[test]
fn reported_cost_round_trips() {
    let mut conn = migrated();
    let events = [
        priced("e1:grok", 10, 1_540_616),
        event("e2", "2026-09-23T09:05:00Z", "gpt-5.5", 1, 1),
    ];
    ingest(&mut conn, &home(), &events, &LogCursors::default()).unwrap();
    assert_eq!(load_since(&conn, &home(), since()).unwrap(), events);
}

#[test]
fn a_larger_event_replaces_the_reported_cost() {
    let mut conn = migrated();
    let cursors = LogCursors::default();
    ingest(&mut conn, &home(), &[priced("e1", 10, 5)], &cursors).unwrap();
    ingest(&mut conn, &home(), &[priced("e1", 5, 1)], &cursors).unwrap();
    let bigger = UsageEvent {
        reported_cost: None,
        ..priced("e1", 20, 0)
    };
    let stored = load_since(&conn, &home(), since()).unwrap();
    assert_eq!(stored[0].reported_cost, Some(MicroUsd(5)));
    ingest(&mut conn, &home(), std::slice::from_ref(&bigger), &cursors).unwrap();
    assert_eq!(load_since(&conn, &home(), since()).unwrap(), [bigger]);
}

#[test]
fn migrating_keeps_old_rows_without_a_reported_cost() {
    let mut conn = version_2_database();
    conn.execute(
        "INSERT INTO usage_events VALUES ('codex', '/home/ada/.grok', 'old', ?1, 'gpt-5.5', \
         'standard', 1, 0, 0, 0, 2, 0, 3, 0)",
        [timestamp_to_sql(ts("2026-09-22T08:00:00Z")).unwrap()],
    )
    .unwrap();
    migrations::migrate(&mut conn).unwrap();
    assert_eq!(migrations::user_version(&conn).unwrap(), 3);
    let stored = load_since(&conn, &home(), since()).unwrap();
    assert_eq!(
        stored,
        [event("old", "2026-09-22T08:00:00Z", "gpt-5.5", 1, 2)]
    );
    assert_eq!(stored[0].reported_cost, None);
}
