use super::*;
use crate::storage::{Storage, settings};

const VERSION_0_5: usize = 8;

fn database_from_0_5(prior_use: &str) -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("headroom.db");
    let mut conn = Connection::open(&path).unwrap();
    migrate_through(&mut conn, VERSION_0_5).unwrap();
    conn.execute_batch(prior_use).unwrap();
    (dir, path)
}

fn onboarding_completed(path: &std::path::Path) -> bool {
    Storage::open(path)
        .unwrap()
        .blocking(|conn| settings::load(conn))
        .unwrap()
        .onboarding
        .completed
}

#[test]
fn upgraded_database_with_accounts_skips_onboarding() {
    let (_dir, path) = database_from_0_5(
        "INSERT INTO accounts (id, provider, home, owner, sort_order, last_seen) \
         VALUES ('claude:default', 'claude', '/home/ada/.claude', 'cli', 0, 0);",
    );
    assert!(onboarding_completed(&path));
}

#[test]
fn upgraded_database_with_only_log_cursors_skips_onboarding() {
    let (_dir, path) = database_from_0_5(
        "INSERT INTO log_cursors (provider, usage_home, payload) \
         VALUES ('codex', '/home/ada/.codex', '{}');",
    );
    assert!(onboarding_completed(&path));
}

#[test]
fn upgraded_database_keeps_an_existing_settings_row() {
    let (_dir, path) = database_from_0_5(
        "INSERT INTO accounts (id, provider, home, owner, sort_order, last_seen) \
         VALUES ('claude:default', 'claude', '/home/ada/.claude', 'cli', 0, 0); \
         INSERT INTO settings (id, payload) VALUES (1, '{\"onboarding\":{\"completed\":false}}');",
    );
    assert!(!onboarding_completed(&path));
}

#[test]
fn unused_database_from_0_5_still_shows_onboarding() {
    let (_dir, path) = database_from_0_5("");
    assert!(!onboarding_completed(&path));
}

#[test]
fn fresh_database_shows_onboarding() {
    let dir = tempfile::tempdir().unwrap();
    assert!(!onboarding_completed(&dir.path().join("headroom.db")));
}

#[test]
fn migrated_onboarding_row_fills_every_other_default() {
    let (_dir, path) = database_from_0_5(
        "INSERT INTO usage_events (provider, usage_home, key, at, model, tier, input, cache_read, \
         cache_write_5m, cache_write_1h, output, reasoning, total, web_search) \
         VALUES ('codex', '/home/ada/.codex', 'r1', 0, 'gpt-5.5', 'standard', 1, 0, 0, 0, 1, 0, 2, 0);",
    );
    let loaded = Storage::open(&path)
        .unwrap()
        .blocking(|conn| settings::load(conn))
        .unwrap();
    let mut expected = crate::settings::Settings::default();
    expected.onboarding.completed = true;
    assert_eq!(loaded, expected);
}
