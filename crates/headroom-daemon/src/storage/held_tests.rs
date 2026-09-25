use super::*;
use crate::storage::Storage;
use crate::testing::ts;

fn held(storage: &Storage, id: &str, at: &str, payload: &str) {
    storage
        .blocking(|conn| save(conn, id, ts(at), &payload))
        .unwrap();
}

fn stored(storage: &Storage) -> Vec<String> {
    storage.blocking(|conn| load_all(conn)).unwrap()
}

#[test]
fn held_alerts_are_ordered_and_replaced_by_id() {
    let storage = Storage::open_in_memory().unwrap();
    held(&storage, "b", "2026-09-23T23:00:00Z", "first b");
    held(&storage, "a", "2026-09-23T23:30:00Z", "a");
    held(&storage, "b", "2026-09-24T01:00:00Z", "latest b");
    assert_eq!(stored(&storage), ["a", "latest b"]);
    storage
        .blocking(|conn| delete(conn, &["b".to_owned(), "missing".to_owned()]))
        .unwrap();
    assert_eq!(stored(&storage), ["a"]);
}

#[test]
fn unreadable_held_alerts_are_dropped_instead_of_failing_the_load() {
    let storage = Storage::open_in_memory().unwrap();
    held(&storage, "a", "2026-09-23T23:00:00Z", "a");
    storage
        .blocking(|conn| {
            conn.execute(
                "INSERT INTO held_alerts (id, held_at, payload) VALUES ('broken', 1, '{not json')",
                [],
            )?;
            Ok(())
        })
        .unwrap();
    held(&storage, "b", "2026-09-24T01:00:00Z", "b");
    assert_eq!(stored(&storage), ["a", "b"]);
    let remaining: i64 = storage
        .blocking(|conn| {
            Ok(conn.query_row("SELECT COUNT(*) FROM held_alerts", [], |row| row.get(0))?)
        })
        .unwrap();
    assert_eq!(remaining, 2);
}

#[test]
fn held_alerts_survive_reopening_the_database() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("headroom.db");
    held(
        &Storage::open(&path).unwrap(),
        "a",
        "2026-09-23T23:00:00Z",
        "a",
    );
    assert_eq!(stored(&Storage::open(&path).unwrap()), ["a"]);
}
