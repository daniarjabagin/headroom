use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use serde_json::json;
use tempfile::TempDir;

use super::*;

fn log_file(content: &str) -> (TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("session.jsonl");
    fs::write(&path, content).unwrap();
    (dir, path)
}

fn append(path: &Path, content: &str) {
    let mut file = OpenOptions::new().append(true).open(path).unwrap();
    file.write_all(content.as_bytes()).unwrap();
}

fn set_mtime(path: &Path, secs: u64) {
    let file = OpenOptions::new().write(true).open(path).unwrap();
    file.set_modified(SystemTime::UNIX_EPOCH + Duration::from_secs(secs))
        .unwrap();
}

fn collect(lines: &mut Vec<String>, line: &str) {
    lines.push(line.to_owned());
}

fn read(path: &Path, cursor: &mut FileCursor) -> Vec<String> {
    read_new_lines(path, cursor, |_| Vec::new(), collect).unwrap()
}

fn read_tracked(path: &Path, cursors: &mut LogCursors) -> Option<Vec<String>> {
    read_new_lines_or_skip(path, cursors, |_| Vec::new(), collect)
}

#[test]
fn first_read_returns_complete_lines_and_holds_partial() {
    let (_dir, path) = log_file("{\"a\":1}\n{\"b\":2}\n{\"c\":");
    let mut cursor = FileCursor::default();
    assert_eq!(read(&path, &mut cursor), ["{\"a\":1}", "{\"b\":2}"]);
    assert_eq!(cursor.offset, 16);
    assert_eq!(cursor.size, 21);
    assert_ne!(cursor.inode, 0);
}

#[test]
fn appended_lines_are_returned_once() {
    let (_dir, path) = log_file("one\n");
    let mut cursor = FileCursor::default();
    assert_eq!(read(&path, &mut cursor), ["one"]);
    append(&path, "two\nthree\n");
    assert_eq!(read(&path, &mut cursor), ["two", "three"]);
    assert!(read(&path, &mut cursor).is_empty());
}

#[test]
fn partial_line_is_completed_on_next_read() {
    let (_dir, path) = log_file("one\ntw");
    let mut cursor = FileCursor::default();
    assert_eq!(read(&path, &mut cursor), ["one"]);
    assert!(read(&path, &mut cursor).is_empty());
    append(&path, "o\n");
    assert_eq!(read(&path, &mut cursor), ["two"]);
    assert_eq!(cursor.offset, 8);
}

#[test]
fn growth_keeps_parser_state() {
    let (_dir, path) = log_file("one\n");
    let mut cursor = FileCursor::default();
    read(&path, &mut cursor);
    cursor.state = json!({ "model": "gpt-5.5" });
    append(&path, "two\n");
    set_mtime(&path, 2_000_000_000);
    assert_eq!(read(&path, &mut cursor), ["two"]);
    assert_eq!(cursor.state, json!({ "model": "gpt-5.5" }));
}

#[test]
fn truncation_resets_cursor_and_state() {
    let (_dir, path) = log_file("a\nb\nc\n");
    let mut cursor = FileCursor::default();
    read(&path, &mut cursor);
    cursor.state = json!({ "prev": 3 });
    fs::write(&path, "x\n").unwrap();
    assert_eq!(read(&path, &mut cursor), ["x"]);
    assert_eq!(cursor.state, serde_json::Value::Null);
    assert_eq!(cursor.offset, 2);
}

#[test]
fn replaced_file_resets_by_inode() {
    let (dir, path) = log_file("a\nb\n");
    let mut cursor = FileCursor::default();
    read(&path, &mut cursor);
    let old_inode = cursor.inode;
    cursor.state = json!({ "prev": 2 });
    let replacement = dir.path().join("replacement.jsonl");
    fs::write(&replacement, "c\nd\ne\n").unwrap();
    fs::rename(&replacement, &path).unwrap();
    assert_eq!(read(&path, &mut cursor), ["c", "d", "e"]);
    assert_ne!(cursor.inode, old_inode);
    assert_eq!(cursor.state, serde_json::Value::Null);
}

#[test]
fn same_size_rewrite_with_new_mtime_resets() {
    let (_dir, path) = log_file("aa\n");
    set_mtime(&path, 1_000_000_000);
    let mut cursor = FileCursor::default();
    read(&path, &mut cursor);
    cursor.state = json!(1);
    fs::write(&path, "bb\n").unwrap();
    set_mtime(&path, 1_000_000_100);
    assert_eq!(read(&path, &mut cursor), ["bb"]);
    assert_eq!(cursor.state, serde_json::Value::Null);
}

#[test]
fn older_mtime_resets() {
    let (_dir, path) = log_file("a\n");
    set_mtime(&path, 1_000_000_000);
    let mut cursor = FileCursor::default();
    read(&path, &mut cursor);
    append(&path, "b\n");
    set_mtime(&path, 900_000_000);
    assert_eq!(read(&path, &mut cursor), ["a", "b"]);
}

#[test]
fn unchanged_file_yields_nothing_and_keeps_state() {
    let (_dir, path) = log_file("a\n");
    let mut cursor = FileCursor::default();
    read(&path, &mut cursor);
    cursor.state = json!(true);
    assert!(read(&path, &mut cursor).is_empty());
    assert_eq!(cursor.state, json!(true));
}

#[test]
fn offset_beyond_size_resets() {
    let (_dir, path) = log_file("a\n");
    let mut cursor = FileCursor::default();
    read(&path, &mut cursor);
    cursor.offset = 99;
    assert_eq!(read(&path, &mut cursor), ["a"]);
}

#[test]
fn crlf_and_blank_lines_are_normalized() {
    let (_dir, path) = log_file("a\r\n\n  \nb\n");
    let mut cursor = FileCursor::default();
    assert_eq!(read(&path, &mut cursor), ["a", "b"]);
    assert_eq!(cursor.offset, 9);
}

#[test]
fn missing_file_is_an_error_with_path() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("gone.jsonl");
    let error = read_new_lines(&path, &mut FileCursor::default(), |_| (), |(), _| {}).unwrap_err();
    assert!(error.to_string().contains("gone.jsonl"));
    let provider_error: headroom_core::provider::ProviderError = error.into();
    assert!(matches!(
        provider_error,
        headroom_core::provider::ProviderError::LocalData(_)
    ));
}

#[test]
fn prune_drops_only_missing_files() {
    let dir = tempfile::tempdir().unwrap();
    let kept = dir.path().join("kept.jsonl");
    let deleted = dir.path().join("deleted.jsonl");
    fs::write(&kept, "").unwrap();
    fs::write(&deleted, "").unwrap();
    let mut cursors = LogCursors::default();
    cursors.cursor_mut(&kept);
    cursors.cursor_mut(&deleted);
    cursors.cursor_mut(&dir.path().join("never.jsonl"));
    fs::remove_file(&deleted).unwrap();
    prune_missing(&mut cursors);
    assert_eq!(cursors.0.keys().collect::<Vec<_>>(), [&kept]);
}

#[test]
fn skipped_read_restores_the_previous_cursor() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("directory.jsonl");
    fs::create_dir(&path).unwrap();
    let mut cursors = LogCursors::default();
    let previous = FileCursor {
        inode: 1,
        size: 10,
        mtime_ns: 5,
        offset: 10,
        state: json!({ "model": "m" }),
    };
    *cursors.cursor_mut(&path) = previous.clone();
    assert_eq!(read_tracked(&path, &mut cursors), None);
    assert_eq!(cursors.0.get(&path), Some(&previous));
}

#[test]
fn skipped_read_of_an_unknown_file_leaves_no_cursor() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("directory.jsonl");
    fs::create_dir(&path).unwrap();
    let mut cursors = LogCursors::default();
    assert_eq!(read_tracked(&path, &mut cursors), None);
    assert!(cursors.0.is_empty());
}

#[test]
fn successful_tracked_read_advances_the_cursor() {
    let (_dir, path) = log_file("a\nb\n");
    let mut cursors = LogCursors::default();
    assert_eq!(
        read_tracked(&path, &mut cursors),
        Some(vec!["a".to_owned(), "b".to_owned()])
    );
    assert_eq!(cursors.0[&path].offset, 4);
}

#[test]
fn init_sees_the_state_after_a_reset() {
    let (_dir, path) = log_file("a\nb\n");
    let mut cursor = FileCursor::default();
    read(&path, &mut cursor);
    cursor.state = json!({ "prev": 2 });
    let kept = read_new_lines(&path, &mut cursor, |c| c.state.clone(), |_, _| {}).unwrap();
    assert_eq!(kept, json!({ "prev": 2 }));
    fs::write(&path, "x\n").unwrap();
    let reset = read_new_lines(&path, &mut cursor, |c| c.state.clone(), |_, _| {}).unwrap();
    assert_eq!(reset, serde_json::Value::Null);
}

#[test]
fn lines_longer_than_the_read_buffer_arrive_whole() {
    let long = "x".repeat(READ_BUFFER * 3 + 17);
    let (_dir, path) = log_file(&format!("{long}\nshort\n{long}"));
    let mut cursor = FileCursor::default();
    assert_eq!(read(&path, &mut cursor), [long.as_str(), "short"]);
    assert_eq!(cursor.offset, (long.len() + 7) as u64);
}

#[test]
fn invalid_utf8_is_replaced_not_dropped() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("bytes.jsonl");
    fs::write(&path, b"a\xffb\n").unwrap();
    let mut cursor = FileCursor::default();
    assert_eq!(read(&path, &mut cursor), ["a\u{fffd}b"]);
}
