use std::path::PathBuf;

use headroom_core::cursor::LogCursors;
use rusqlite::Connection;

use super::*;
use crate::storage::migrations;
use crate::testing::{CLAUDE, CODEX, event, ts};

const APP: &str = "/home/user/work/app";

const VERSION_5: [&str; 5] = [
    include_str!("migrations/001_init.sql"),
    include_str!("migrations/002_subscription_lapses.sql"),
    include_str!("migrations/003_reported_cost.sql"),
    include_str!("migrations/004_dismissed_homes.sql"),
    include_str!("migrations/005_update_check.sql"),
];

fn claude_home() -> UsageHome {
    UsageHome {
        provider: CLAUDE,
        home: PathBuf::from("/home/user/.claude"),
    }
}

fn version_5_database() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    for sql in VERSION_5 {
        conn.execute_batch(sql).unwrap();
    }
    conn.pragma_update(None, "user_version", 5).unwrap();
    conn
}

fn migrated() -> Connection {
    let mut conn = Connection::open_in_memory().unwrap();
    migrations::migrate(&mut conn).unwrap();
    conn
}

fn placed(key: &str, output: u64, project: Option<&str>) -> UsageEvent {
    UsageEvent {
        project: project.map(str::to_owned),
        ..event(key, "2026-09-23T09:00:00Z", "claude-opus-5-5", 100, output)
    }
}

fn load(conn: &Connection) -> Vec<UsageEvent> {
    load_since(conn, &claude_home(), ts("2026-09-01T00:00:00Z")).unwrap()
}

fn store_one(conn: &mut Connection, event: &UsageEvent) -> usize {
    let events = std::slice::from_ref(event);
    ingest(conn, &claude_home(), events, &LogCursors::default())
        .unwrap()
        .changed
}

#[test]
fn project_round_trips() {
    let mut conn = migrated();
    let events = [placed("a", 1, Some(APP)), placed("b", 2, None)];
    ingest(&mut conn, &claude_home(), &events, &LogCursors::default()).unwrap();
    assert_eq!(load(&conn), events);
}

#[test]
fn reingesting_fills_a_missing_project_without_touching_the_numbers() {
    let mut conn = version_5_database();
    conn.execute(
        "INSERT INTO usage_events VALUES ('claude', '/home/user/.claude', 'a', ?1, \
         'claude-opus-5-5', 'standard', 100, 0, 0, 0, 20, 0, 120, 0, NULL)",
        [timestamp_to_sql(ts("2026-09-23T09:00:00Z")).unwrap()],
    )
    .unwrap();
    migrations::migrate(&mut conn).unwrap();
    assert_eq!(load(&conn), [placed("a", 20, None)]);
    let rows = [
        (placed("a", 20, Some(APP)), 1, Some(APP), 20),
        (placed("a", 20, Some(APP)), 0, Some(APP), 20),
        (placed("a", 20, Some("/srv/other")), 0, Some(APP), 20),
        (placed("a", 20, None), 0, Some(APP), 20),
    ];
    for (incoming, changed, project, output) in rows {
        assert_eq!(store_one(&mut conn, &incoming), changed, "{incoming:?}");
        let stored = load(&conn);
        assert_eq!(stored.len(), 1);
        assert_eq!(stored[0].project.as_deref(), project);
        assert_eq!(stored[0].tokens.output.0, output);
        assert_eq!(stored[0].tokens.total().0, 100 + output);
    }
}

#[test]
fn a_smaller_chunk_fills_the_project_but_keeps_the_larger_counts() {
    let mut conn = migrated();
    store_one(&mut conn, &placed("a", 50, None));
    assert_eq!(store_one(&mut conn, &placed("a", 10, Some(APP))), 1);
    assert_eq!(load(&conn), [placed("a", 50, Some(APP))]);
}

#[test]
fn a_larger_chunk_without_a_project_keeps_the_stored_one() {
    let mut conn = migrated();
    store_one(&mut conn, &placed("a", 10, Some(APP)));
    assert_eq!(store_one(&mut conn, &placed("a", 50, None)), 1);
    assert_eq!(load(&conn), [placed("a", 50, Some(APP))]);
    assert_eq!(store_one(&mut conn, &placed("a", 60, Some("/srv/new"))), 1);
    assert_eq!(load(&conn), [placed("a", 60, Some("/srv/new"))]);
}

fn saved_cursors(conn: &Connection, provider: &str) -> String {
    conn.query_row(
        "SELECT payload FROM log_cursors WHERE provider = ?1",
        [provider],
        |row| row.get(0),
    )
    .unwrap()
}

#[test]
fn migrating_resets_claude_and_codex_cursors_but_keeps_their_homes() {
    let conn = version_5_database();
    let cursor = r#"{"/home/user/.claude/projects/p/s.jsonl":{"inode":1,"size":9,"mtime_ns":1,"offset":9,"state":null}}"#;
    for provider in ["claude", "codex", "grok"] {
        conn.execute(
            "INSERT INTO log_cursors (provider, usage_home, payload) VALUES (?1, ?2, ?3)",
            [provider, &format!("/home/user/.{provider}"), cursor],
        )
        .unwrap();
    }
    let mut conn = conn;
    migrations::migrate(&mut conn).unwrap();
    assert_eq!(saved_cursors(&conn, "claude"), "{}");
    assert_eq!(saved_cursors(&conn, "codex"), "{}");
    assert_eq!(saved_cursors(&conn, "grok"), cursor);
    assert_eq!(
        cursors::load(&conn, &claude_home()).unwrap(),
        LogCursors::default()
    );
    let homes = cursors::homes(&conn).unwrap();
    assert_eq!(homes.len(), 3);
    assert!(homes.iter().any(|home| home.provider == CODEX));
}
