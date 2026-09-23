use std::fs::{self, OpenOptions};
use std::io::Write;

use super::super::test_support::{at, write_rollout};
use super::*;
use crate::jsonl::locked::Locked;

const RECORDS: &str = include_str!("fixtures/rollout_records.jsonl");
const LEGACY: &str = include_str!("fixtures/rollout_legacy.jsonl");
const NOW: &str = "2026-12-01T00:00:00Z";

fn append(path: &Path, content: &str) {
    let mut file = OpenOptions::new().append(true).open(path).unwrap();
    file.write_all(content.as_bytes()).unwrap();
}

fn keys(events: &[UsageEvent]) -> Vec<String> {
    events.iter().map(|event| event.key.0.clone()).collect()
}

#[test]
fn reads_sessions_and_archived_sessions() {
    let home = tempfile::tempdir().unwrap();
    write_rollout(home.path(), "sessions/2026/09/22/rollout-a.jsonl", RECORDS);
    write_rollout(home.path(), "archived_sessions/rollout-b.jsonl", LEGACY);
    write_rollout(home.path(), "history.jsonl", RECORDS);
    let mut cursors = LogCursors::default();
    let events = read_usage(home.path(), &mut cursors, at(NOW)).unwrap();
    assert_eq!(events.len(), 5);
    assert_eq!(cursors.0.len(), 2);
    let fallback = events.iter().filter(|event| event.key.is_fallback());
    assert_eq!(fallback.count(), 2);
}

#[test]
fn second_read_returns_only_new_events() {
    let home = tempfile::tempdir().unwrap();
    write_rollout(home.path(), "sessions/rollout.jsonl", RECORDS);
    let mut cursors = LogCursors::default();
    assert_eq!(
        read_usage(home.path(), &mut cursors, at(NOW))
            .unwrap()
            .len(),
        3
    );
    assert!(
        read_usage(home.path(), &mut cursors, at(NOW))
            .unwrap()
            .is_empty()
    );
}

#[test]
fn appended_partial_line_is_parsed_once_complete() {
    let home = tempfile::tempdir().unwrap();
    let lines: Vec<&str> = RECORDS.lines().collect();
    let first = format!("{}\n", lines[..6].join("\n"));
    let record_two = lines[8];
    let (head, tail) = record_two.split_at(record_two.len() / 2);
    let path = write_rollout(
        home.path(),
        "sessions/rollout.jsonl",
        &format!("{first}{head}"),
    );
    let mut cursors = LogCursors::default();
    let first_read = read_usage(home.path(), &mut cursors, at(NOW)).unwrap();
    assert_eq!(keys(&first_read), ["resp_fake_0001"]);
    append(&path, &format!("{tail}\n{}\n", lines[9..].join("\n")));
    let second_read = read_usage(home.path(), &mut cursors, at(NOW)).unwrap();
    assert_eq!(keys(&second_read), ["resp_fake_0002", "resp_fake_0003"]);
    assert_eq!(second_read[0].model, "gpt-5.5");
    assert_eq!(
        second_read[0].tier,
        headroom_core::event::ServiceTier::Priority
    );
}

#[test]
fn pending_token_counts_are_released_on_a_later_read() {
    let home = tempfile::tempdir().unwrap();
    write_rollout(home.path(), "sessions/legacy.jsonl", LEGACY);
    let mut cursors = LogCursors::default();
    let early = read_usage(home.path(), &mut cursors, at("2026-07-10T08:05:00Z")).unwrap();
    assert!(early.is_empty());
    let later = read_usage(home.path(), &mut cursors, at(NOW)).unwrap();
    assert_eq!(later.len(), 2);
    assert!(
        read_usage(home.path(), &mut cursors, at(NOW))
            .unwrap()
            .is_empty()
    );
}

#[test]
fn deleted_files_are_pruned_from_cursors() {
    let home = tempfile::tempdir().unwrap();
    let path = write_rollout(home.path(), "sessions/rollout.jsonl", RECORDS);
    let mut cursors = LogCursors::default();
    read_usage(home.path(), &mut cursors, at(NOW)).unwrap();
    fs::remove_file(&path).unwrap();
    assert!(
        read_usage(home.path(), &mut cursors, at(NOW))
            .unwrap()
            .is_empty()
    );
    assert!(cursors.0.is_empty());
}

#[test]
fn missing_home_has_no_usage() {
    let home = tempfile::tempdir().unwrap();
    let mut cursors = LogCursors::default();
    let events = read_usage(&home.path().join("absent"), &mut cursors, at(NOW)).unwrap();
    assert!(events.is_empty());
}

#[test]
fn unreadable_rollout_is_skipped_without_losing_other_events() {
    let home = tempfile::tempdir().unwrap();
    let locked = write_rollout(home.path(), "sessions/a/rollout-a.jsonl", LEGACY);
    write_rollout(home.path(), "sessions/b/rollout-b.jsonl", RECORDS);
    let mut cursors = LogCursors::default();
    let lock = Locked::new(&locked);
    if !lock.is_enforced() {
        return;
    }
    let events = read_usage(home.path(), &mut cursors, at(NOW)).unwrap();
    assert_eq!(events.len(), 3);
    assert!(!cursors.0.contains_key(&locked));
    drop(lock);
    let events = read_usage(home.path(), &mut cursors, at(NOW)).unwrap();
    assert_eq!(events.len(), 2);
}

#[test]
fn unreadable_session_directory_is_skipped() {
    let home = tempfile::tempdir().unwrap();
    write_rollout(home.path(), "sessions/a/rollout-a.jsonl", LEGACY);
    write_rollout(home.path(), "sessions/b/rollout-b.jsonl", RECORDS);
    let lock = Locked::new(&home.path().join("sessions/a"));
    if !lock.is_enforced() {
        return;
    }
    let events = read_usage(home.path(), &mut LogCursors::default(), at(NOW)).unwrap();
    assert_eq!(events.len(), 3);
}
