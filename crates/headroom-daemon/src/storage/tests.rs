use std::path::{Path, PathBuf};

use headroom_core::account::{AccountId, ProviderKind};
use headroom_core::cursor::LogCursors;
use headroom_core::units::Tokens;
use jiff::Timestamp;

use super::*;
use crate::home::UsageHome;
use crate::settings::Settings;
use crate::testing::{account, event, session, snapshot, ts};

fn memory() -> Storage {
    Storage::open_in_memory().unwrap()
}

fn codex_home() -> UsageHome {
    UsageHome {
        provider: ProviderKind::Codex,
        home: PathBuf::from("/home/ada/.codex"),
    }
}

fn table_names(storage: &Storage) -> Vec<String> {
    storage
        .blocking(|conn| {
            let mut statement =
                conn.prepare("SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name")?;
            let names = statement
                .query_map([], |row| row.get(0))?
                .collect::<Result<Vec<String>, _>>()?;
            Ok(names)
        })
        .unwrap()
}

#[test]
fn fresh_database_is_migrated_to_latest() {
    let storage = memory();
    let version = storage
        .blocking(|conn| migrations::user_version(conn))
        .unwrap();
    assert_eq!(version, migrations::latest());
    assert_eq!(
        table_names(&storage),
        [
            "accounts",
            "limits_snapshots",
            "log_cursors",
            "notification_state",
            "settings",
            "subscription_lapses",
            "usage_events"
        ]
    );
}

#[test]
fn reopening_a_file_database_keeps_data_and_uses_wal() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("nested/headroom.db");
    let first = Storage::open(&path).unwrap();
    first
        .blocking(|conn| {
            settings::save(
                conn,
                &Settings {
                    reduced_motion: true,
                    ..Settings::default()
                },
            )
        })
        .unwrap();
    drop(first);
    let second = Storage::open(&path).unwrap();
    let mode: String = second
        .blocking(|conn| Ok(conn.pragma_query_value(None, "journal_mode", |row| row.get(0))?))
        .unwrap();
    assert_eq!(mode, "wal");
    assert!(
        second
            .blocking(|conn| settings::load(conn))
            .unwrap()
            .reduced_motion
    );
}

#[test]
fn the_page_cache_is_bounded() {
    let cache: i64 = memory()
        .blocking(|conn| Ok(conn.pragma_query_value(None, "cache_size", |row| row.get(0))?))
        .unwrap();
    assert_eq!(cache, -PAGE_CACHE_KIB);
}

#[test]
fn newer_schema_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("headroom.db");
    drop(Storage::open(&path).unwrap());
    let conn = rusqlite::Connection::open(&path).unwrap();
    conn.pragma_update(None, "user_version", 99).unwrap();
    drop(conn);
    assert!(matches!(
        Storage::open(&path),
        Err(StorageError::FutureSchema { found: 99, .. })
    ));
}

#[test]
fn upsert_keeps_the_event_with_the_larger_total() {
    let storage = memory();
    let home = codex_home();
    let cursors = LogCursors::default();
    let big = event("r1", "2026-09-23T09:00:00Z", "gpt-5.5", 100, 50);
    let small = event("r1", "2026-09-23T09:00:01Z", "gpt-5.5", 100, 10);
    let bigger = event("r1", "2026-09-23T09:00:02Z", "gpt-5.5", 100, 90);
    let changes = storage
        .blocking(|conn| {
            let first = events::ingest(conn, &home, std::slice::from_ref(&big), &cursors)?;
            let second = events::ingest(conn, &home, &[small], &cursors)?;
            Ok((first, second))
        })
        .unwrap();
    assert_eq!(changes, (1, 0));
    let since = ts("2026-09-01T00:00:00Z");
    let stored = storage
        .blocking(|conn| events::load_since(conn, &home, since))
        .unwrap();
    assert_eq!(stored, [big]);
    storage
        .blocking(|conn| events::ingest(conn, &home, std::slice::from_ref(&bigger), &cursors))
        .unwrap();
    let stored = storage
        .blocking(|conn| events::load_since(conn, &home, since))
        .unwrap();
    assert_eq!(stored, [bigger]);
}

#[test]
fn events_are_scoped_by_provider_and_home() {
    let storage = memory();
    let other = UsageHome {
        provider: ProviderKind::Codex,
        home: PathBuf::from("/srv/codex"),
    };
    let cursors = LogCursors::default();
    let one = event("r1", "2026-09-23T09:00:00Z", "gpt-5.5", 1, 1);
    storage
        .blocking(|conn| {
            events::ingest(conn, &codex_home(), std::slice::from_ref(&one), &cursors)?;
            events::ingest(conn, &other, std::slice::from_ref(&one), &cursors)
        })
        .unwrap();
    let since = ts("2026-09-01T00:00:00Z");
    let stored = storage
        .blocking(|conn| events::load_since(conn, &other, since))
        .unwrap();
    assert_eq!(stored, [one]);
}

#[test]
fn prune_removes_only_events_before_the_cutoff() {
    let storage = memory();
    let home = codex_home();
    let old = event("old", "2026-08-01T00:00:00Z", "gpt-5.5", 1, 1);
    let recent = event("new", "2026-09-20T00:00:00Z", "gpt-5.5", 1, 1);
    storage
        .blocking(|conn| {
            events::ingest(conn, &home, &[old, recent.clone()], &LogCursors::default())
        })
        .unwrap();
    let removed = storage
        .blocking(|conn| events::prune_before(conn, ts("2026-08-19T10:00:00Z")))
        .unwrap();
    assert_eq!(removed, 1);
    let stored = storage
        .blocking(|conn| events::load_since(conn, &home, Timestamp::UNIX_EPOCH))
        .unwrap();
    assert_eq!(stored, [recent]);
}

#[test]
fn cursors_persist_with_events() {
    let storage = memory();
    let home = codex_home();
    let mut cursors = LogCursors::default();
    cursors
        .cursor_mut(Path::new("/home/ada/.codex/s.jsonl"))
        .offset = 42;
    storage
        .blocking(|conn| events::ingest(conn, &home, &[], &cursors))
        .unwrap();
    assert_eq!(
        storage.blocking(|conn| cursors::load(conn, &home)).unwrap(),
        cursors
    );
}

#[test]
fn failed_ingest_rolls_back_events_and_cursors() {
    let storage = memory();
    let home = codex_home();
    let mut cursors = LogCursors::default();
    cursors
        .cursor_mut(Path::new("/home/ada/.codex/s.jsonl"))
        .offset = 42;
    let good = event("ok", "2026-09-23T09:00:00Z", "gpt-5.5", 1, 1);
    let mut huge = event("huge", "2026-09-23T09:00:00Z", "gpt-5.5", 1, 1);
    huge.tokens.input = Tokens(u64::MAX);
    let result = storage.blocking(|conn| events::ingest(conn, &home, &[good, huge], &cursors));
    assert!(matches!(result, Err(StorageError::OutOfRange(_))));
    let stored = storage
        .blocking(|conn| events::load_since(conn, &home, Timestamp::UNIX_EPOCH))
        .unwrap();
    assert!(stored.is_empty());
    assert_eq!(
        storage.blocking(|conn| cursors::load(conn, &home)).unwrap(),
        LogCursors::default()
    );
}

#[test]
fn sync_adds_accounts_in_discovery_order_and_marks_vanished_gone() {
    let storage = memory();
    let work = account(ProviderKind::Codex, "work");
    let home = account(ProviderKind::Codex, "home");
    let claude = account(ProviderKind::Claude, "main");
    let now = ts("2026-09-23T10:00:00Z");
    storage
        .blocking(|conn| {
            accounts::sync_provider(
                conn,
                ProviderKind::Codex,
                &[work.clone(), home.clone()],
                now,
            )?;
            accounts::sync_provider(
                conn,
                ProviderKind::Claude,
                std::slice::from_ref(&claude),
                now,
            )?;
            accounts::set_label(conn, &work.id, Some("Work"))?;
            accounts::sync_provider(conn, ProviderKind::Codex, std::slice::from_ref(&work), now)
        })
        .unwrap();
    let stored = storage.blocking(|conn| accounts::load_all(conn)).unwrap();
    let summary: Vec<_> = stored
        .iter()
        .map(|a| (a.id().0.as_str(), a.sort_order, a.label.as_deref(), a.gone))
        .collect();
    assert_eq!(
        summary,
        [
            ("codex:work", 0, Some("Work"), false),
            ("codex:home", 1, None, true),
            ("claude:main", 2, None, false)
        ]
    );
}

#[test]
fn rediscovered_account_comes_back_with_its_settings() {
    let storage = memory();
    let work = account(ProviderKind::Codex, "work");
    let now = ts("2026-09-23T10:00:00Z");
    storage
        .blocking(|conn| {
            accounts::sync_provider(conn, ProviderKind::Codex, std::slice::from_ref(&work), now)?;
            accounts::set_hidden(conn, &work.id, true)?;
            accounts::sync_provider(conn, ProviderKind::Codex, &[], now)?;
            accounts::sync_provider(conn, ProviderKind::Codex, std::slice::from_ref(&work), now)
        })
        .unwrap();
    let stored = storage.blocking(|conn| accounts::load_all(conn)).unwrap();
    assert_eq!(stored.len(), 1);
    assert!(stored[0].hidden);
    assert!(!stored[0].gone);
}

#[test]
fn order_identity_and_unknown_accounts() {
    let storage = memory();
    let a = account(ProviderKind::Codex, "a");
    let b = account(ProviderKind::Codex, "b");
    let now = ts("2026-09-23T10:00:00Z");
    let unknown = AccountId("codex:none".into());
    let found = storage
        .blocking(|conn| {
            accounts::sync_provider(conn, ProviderKind::Codex, &[a.clone(), b.clone()], now)?;
            accounts::set_order(conn, &[b.id.clone(), a.id.clone()])?;
            accounts::set_identity(conn, &a.id, Some("a@example.com"), Some("Plus"))?;
            accounts::set_label(conn, &unknown, Some("x"))
        })
        .unwrap();
    assert!(!found);
    let stored = storage.blocking(|conn| accounts::load_all(conn)).unwrap();
    assert_eq!(stored[0].id(), &b.id);
    assert_eq!(stored[1].email.as_deref(), Some("a@example.com"));
    assert_eq!(stored[1].plan.as_deref(), Some("Plus"));
}

#[test]
fn snapshots_round_trip() {
    let storage = memory();
    let id = AccountId("codex:work".into());
    let first = snapshot(
        vec![session(10.0, "2026-09-23T12:00:00Z")],
        "2026-09-23T10:00:00Z",
    );
    let second = snapshot(
        vec![session(20.0, "2026-09-23T12:00:00Z")],
        "2026-09-23T10:05:00Z",
    );
    storage
        .blocking(|conn| {
            snapshots::save(conn, &id, &first)?;
            snapshots::save(conn, &id, &second)
        })
        .unwrap();
    let stored = storage.blocking(|conn| snapshots::load_all(conn)).unwrap();
    assert_eq!(stored, [(id, second)]);
}

#[test]
fn settings_default_when_absent() {
    let storage = memory();
    assert_eq!(
        storage.blocking(|conn| settings::load(conn)).unwrap(),
        Settings::default()
    );
}

#[test]
fn legacy_settings_rows_are_migrated_on_load() {
    let storage = memory();
    let loaded = storage
        .blocking(|conn| {
            conn.execute(
                "INSERT INTO settings (id, payload) VALUES (1, ?1)",
                [r#"{"refresh_interval_secs":120,"show_usage":false}"#],
            )?;
            settings::load(conn)
        })
        .unwrap();
    assert_eq!(loaded.refresh_interval_secs, 120);
    assert!(!loaded.display.show_spend);
}

#[test]
fn alert_payloads_round_trip() {
    let storage = memory();
    let id = AccountId("codex:work".into());
    storage
        .blocking(|conn| alerts::save(conn, &id, "session", &vec![1_u8, 2]))
        .unwrap();
    let stored: Vec<(AccountId, String, Vec<u8>)> =
        storage.blocking(|conn| alerts::load_all(conn)).unwrap();
    assert_eq!(stored, [(id, "session".to_owned(), vec![1, 2])]);
}

#[test]
fn deleted_snapshots_are_gone() {
    let storage = memory();
    let id = AccountId("codex:work".into());
    let stored = snapshot(Vec::new(), "2026-09-23T10:00:00Z");
    storage
        .blocking(|conn| {
            snapshots::save(conn, &id, &stored)?;
            snapshots::delete(conn, &id)
        })
        .unwrap();
    assert!(
        storage
            .blocking(|conn| snapshots::load_all(conn))
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn run_executes_on_the_blocking_pool() {
    let storage = memory();
    let count = storage
        .run(|conn| Ok(accounts::load_all(conn)?.len()))
        .await
        .unwrap();
    assert_eq!(count, 0);
}
