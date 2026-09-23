use std::fs::{self, OpenOptions};
use std::io::Write;

use headroom_core::units::MicroUsd;
use tempfile::TempDir;

use super::*;

const UPDATES: &str = include_str!("fixtures/updates.jsonl");
const FORKED: &str = include_str!("fixtures/forked.jsonl");

fn session_file(home: &Path, session: &str, name: &str) -> PathBuf {
    home.join("sessions/%2Fhome%2Fada%2Fproject")
        .join(session)
        .join(name)
}

fn write(path: &Path, content: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

fn append(path: &Path, content: &str) {
    let mut file = OpenOptions::new().append(true).open(path).unwrap();
    file.write_all(content.as_bytes()).unwrap();
}

fn home_with_sessions() -> TempDir {
    let home = tempfile::tempdir().unwrap();
    write(
        &session_file(home.path(), "session-1", UPDATES_FILE),
        UPDATES,
    );
    write(
        &session_file(home.path(), "session-2", UPDATES_FILE),
        FORKED,
    );
    write(
        &session_file(home.path(), "session-1", "chat_history.jsonl"),
        FORKED,
    );
    home
}

fn keys(events: &[UsageEvent]) -> Vec<&str> {
    events.iter().map(|event| event.key.0.as_str()).collect()
}

#[test]
fn replayed_turns_are_counted_once_across_sessions() {
    let home = home_with_sessions();
    let events = read_usage(home.path(), &mut LogCursors::default()).unwrap();
    assert_eq!(
        keys(&events),
        [
            "01a0f000-0000-7000-8000-000000000001-3345:grok-4.6-build",
            "01a0f000-0000-7000-8000-000000000001-3402:grok-4.6-build",
            "01a0f000-0000-7000-8000-000000000002-4401:grok-4.7-build",
            "01a0f000-0000-7000-8000-000000000002-4401:grok-code-fast",
        ]
    );
    let cost: MicroUsd = events.iter().filter_map(|event| event.reported_cost).sum();
    assert_eq!(cost, MicroUsd(1_540_616 + 22_500 + 10_447_922));
}

#[test]
fn only_new_complete_lines_are_read_on_the_next_pass() {
    let home = home_with_sessions();
    let mut cursors = LogCursors::default();
    read_usage(home.path(), &mut cursors).unwrap();
    assert!(read_usage(home.path(), &mut cursors).unwrap().is_empty());
    let path = session_file(home.path(), "session-1", UPDATES_FILE);
    let line = FORKED.lines().nth(1).unwrap();
    let (head, tail) = line.split_at(line.len() / 2);
    append(&path, head);
    assert!(read_usage(home.path(), &mut cursors).unwrap().is_empty());
    append(&path, &format!("{tail}\n"));
    let events = read_usage(home.path(), &mut cursors).unwrap();
    assert_eq!(events.len(), 2);
    assert_eq!(events[0].model, "grok-4.7-build");
}

#[test]
fn a_rewritten_file_is_read_again_from_the_start() {
    let home = home_with_sessions();
    let mut cursors = LogCursors::default();
    read_usage(home.path(), &mut cursors).unwrap();
    let path = session_file(home.path(), "session-2", UPDATES_FILE);
    fs::remove_file(&path).unwrap();
    write(&path, FORKED.lines().next().unwrap());
    append(&path, "\n");
    let events = read_usage(home.path(), &mut cursors).unwrap();
    assert_eq!(
        keys(&events),
        ["01a0f000-0000-7000-8000-000000000001-3345:grok-4.6-build"]
    );
}

#[test]
fn deleted_sessions_drop_their_cursors() {
    let home = home_with_sessions();
    let mut cursors = LogCursors::default();
    read_usage(home.path(), &mut cursors).unwrap();
    assert_eq!(cursors.0.len(), 2);
    fs::remove_dir_all(
        home.path()
            .join("sessions/%2Fhome%2Fada%2Fproject/session-2"),
    )
    .unwrap();
    read_usage(home.path(), &mut cursors).unwrap();
    assert_eq!(cursors.0.len(), 1);
}

#[test]
fn a_home_without_sessions_has_no_usage() {
    let home = tempfile::tempdir().unwrap();
    let mut cursors = LogCursors::default();
    assert!(read_usage(home.path(), &mut cursors).unwrap().is_empty());
    assert!(cursors.0.is_empty());
}
