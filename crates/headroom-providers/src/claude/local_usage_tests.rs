use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

use headroom_core::event::ServiceTier;
use headroom_core::units::Tokens;
use tempfile::TempDir;

use super::*;
use crate::jsonl::locked::Locked;

const SESSION: &str = include_str!("fixtures/session.jsonl");
const SUBAGENT: &str = include_str!("fixtures/subagent.jsonl");
const ITERATIONS: &str = include_str!("fixtures/iterations.jsonl");

fn session_path(home: &Path) -> PathBuf {
    home.join("projects/-home-user-project/session-0001.jsonl")
}

fn subagent_path(home: &Path) -> PathBuf {
    home.join("projects/-home-user-project/session-0001/subagents/agent-fake-1.jsonl")
}

fn write(path: &Path, content: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

fn append(path: &Path, content: &str) {
    let mut file = OpenOptions::new().append(true).open(path).unwrap();
    file.write_all(content.as_bytes()).unwrap();
}

fn home_with_logs() -> TempDir {
    let home = tempfile::tempdir().unwrap();
    write(&session_path(home.path()), SESSION);
    write(&subagent_path(home.path()), SUBAGENT);
    home
}

fn keys(events: &[UsageEvent]) -> Vec<String> {
    events
        .iter()
        .map(|e| {
            if e.key.is_fallback() {
                "fallback".to_owned()
            } else {
                e.key.0.clone()
            }
        })
        .collect()
}

#[test]
fn reads_session_and_subagent_logs_with_one_event_per_key() {
    let home = home_with_logs();
    let events = read_usage(home.path(), &mut LogCursors::default()).unwrap();
    assert_eq!(
        keys(&events),
        [
            "msg_fake_A:req_fake_A",
            "msg_fake_B:req_fake_B",
            "msg_fake_D",
            "msg_fake_S:req_fake_S",
        ]
    );
}

#[test]
fn streaming_duplicates_keep_the_largest_usage() {
    let home = home_with_logs();
    let events = read_usage(home.path(), &mut LogCursors::default()).unwrap();
    let first = &events[0];
    assert_eq!(first.tokens.output, Tokens(332));
    assert_eq!(first.tokens.reasoning, Tokens(81));
    assert_eq!(first.tokens.cache_write_1h, Tokens(2_000));
    assert_eq!(first.tokens.total(), Tokens(3 + 15_000 + 2_000 + 332));
}

#[test]
fn subagent_usage_is_included() {
    let home = home_with_logs();
    let events = read_usage(home.path(), &mut LogCursors::default()).unwrap();
    let subagent = events.last().unwrap();
    assert_eq!(subagent.model, "claude-fable-5-1");
    assert_eq!(subagent.tokens.cache_write_1h, Tokens(1_500));
    assert_eq!(subagent.tier, ServiceTier::Standard);
}

#[test]
fn second_read_without_changes_is_empty() {
    let home = home_with_logs();
    let mut cursors = LogCursors::default();
    read_usage(home.path(), &mut cursors).unwrap();
    assert!(read_usage(home.path(), &mut cursors).unwrap().is_empty());
    assert_eq!(cursors.0.len(), 2);
}

#[test]
fn appends_are_read_incrementally_across_a_partial_line() {
    let home = tempfile::tempdir().unwrap();
    let path = session_path(home.path());
    let lines: Vec<&str> = SESSION.lines().collect();
    let third = lines[2];
    let (head, tail) = third.split_at(third.len() / 2);
    write(&path, &format!("{}\n{}\n{head}", lines[0], lines[1]));
    let mut cursors = LogCursors::default();

    let first = read_usage(home.path(), &mut cursors).unwrap();
    assert_eq!(keys(&first), ["msg_fake_A:req_fake_A"]);
    assert_eq!(first[0].tokens.output, Tokens(12));

    append(&path, &format!("{tail}\n{}\n", lines[5]));
    let second = read_usage(home.path(), &mut cursors).unwrap();
    assert_eq!(
        keys(&second),
        ["msg_fake_A:req_fake_A", "msg_fake_B:req_fake_B"]
    );
    assert_eq!(second[0].tokens.output, Tokens(332));
    assert_eq!(second[1].web_search_requests, 2);
}

#[test]
fn missing_projects_dir_yields_nothing() {
    let home = tempfile::tempdir().unwrap();
    assert!(
        read_usage(home.path(), &mut LogCursors::default())
            .unwrap()
            .is_empty()
    );
}

#[test]
fn cursors_of_deleted_files_are_pruned() {
    let home = home_with_logs();
    let mut cursors = LogCursors::default();
    read_usage(home.path(), &mut cursors).unwrap();
    fs::remove_file(subagent_path(home.path())).unwrap();
    read_usage(home.path(), &mut cursors).unwrap();
    assert_eq!(
        cursors.0.keys().collect::<Vec<_>>(),
        [&session_path(home.path())]
    );
}

#[test]
fn unreadable_file_is_skipped_and_read_once_readable() {
    let home = home_with_logs();
    let mut cursors = LogCursors::default();
    let lock = Locked::new(&session_path(home.path()));
    if !lock.is_enforced() {
        return;
    }
    let events = read_usage(home.path(), &mut cursors).unwrap();
    assert_eq!(keys(&events), ["msg_fake_S:req_fake_S"]);
    assert!(!cursors.0.contains_key(&session_path(home.path())));
    drop(lock);
    let events = read_usage(home.path(), &mut cursors).unwrap();
    assert_eq!(keys(&events)[0], "msg_fake_A:req_fake_A");
    assert_eq!(events.len(), 3);
}

#[test]
fn unreadable_file_keeps_its_existing_cursor() {
    let home = home_with_logs();
    let mut cursors = LogCursors::default();
    read_usage(home.path(), &mut cursors).unwrap();
    let before = cursors.0[&subagent_path(home.path())].clone();
    append(
        &subagent_path(home.path()),
        &SUBAGENT.replace("msg_fake_S", "msg_fake_T"),
    );
    let lock = Locked::new(&subagent_path(home.path()));
    if !lock.is_enforced() {
        return;
    }
    assert!(read_usage(home.path(), &mut cursors).unwrap().is_empty());
    assert_eq!(cursors.0[&subagent_path(home.path())], before);
    drop(lock);
    let events = read_usage(home.path(), &mut cursors).unwrap();
    assert_eq!(keys(&events), ["msg_fake_T:req_fake_S"]);
}

#[test]
fn unreadable_subdirectory_is_skipped() {
    let home = home_with_logs();
    let subagents = subagent_path(home.path()).parent().unwrap().to_path_buf();
    let lock = Locked::new(&subagents);
    if !lock.is_enforced() {
        return;
    }
    let events = read_usage(home.path(), &mut LogCursors::default()).unwrap();
    assert_eq!(events.len(), 3);
    assert!(!keys(&events).contains(&"msg_fake_S:req_fake_S".to_owned()));
}

#[test]
fn streaming_lines_without_request_id_keep_one_event_per_message() {
    let home = tempfile::tempdir().unwrap();
    let lines: Vec<String> = SESSION
        .lines()
        .skip(1)
        .take(3)
        .map(|line| line.replace("\"requestId\":\"req_fake_A\",", ""))
        .collect();
    assert!(lines.iter().all(|line| !line.contains("requestId")));
    write(&session_path(home.path()), &(lines.join("\n") + "\n"));
    let events = read_usage(home.path(), &mut LogCursors::default()).unwrap();
    assert_eq!(keys(&events), ["msg_fake_A"]);
    assert_eq!(events[0].tokens.output, Tokens(332));
}

#[test]
fn iterations_are_deduplicated_across_streaming_lines() {
    let home = tempfile::tempdir().unwrap();
    write(&session_path(home.path()), ITERATIONS);
    let events = read_usage(home.path(), &mut LogCursors::default()).unwrap();
    let summary: Vec<(String, &str, Tokens)> = events
        .iter()
        .map(|e| (e.key.0.clone(), e.model.as_str(), e.tokens.total()))
        .collect();
    assert_eq!(
        summary,
        [
            (
                "msg_fake_F:req_fake_F".to_owned(),
                "claude-opus-4-8",
                Tokens(2 + 3_696 + 5)
            ),
            (
                "msg_fake_F:req_fake_F:iter:0".to_owned(),
                "claude-fable-5-1",
                Tokens(47 + 3_697)
            ),
            (
                "msg_fake_G:req_fake_G".to_owned(),
                "claude-opus-5-5",
                Tokens(2 + 24_000 + 1_600 + 287)
            ),
        ]
    );
}
