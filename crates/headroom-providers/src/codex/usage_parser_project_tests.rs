use serde_json::json;

use super::tests::{LEGACY, LONG_AFTER, RECORDS, line, parse, parse_with, usage, usage_record};
use super::*;

const APP: &str = "/home/user/work/app";
const API: &str = "/home/user/work/api";

fn session_meta(cwd: &serde_json::Value) -> String {
    line(&json!({
        "timestamp": "2026-09-22T10:00:00Z",
        "type": "session_meta",
        "payload": { "id": "00000000-0000-7000-8000-000000000099", "cwd": cwd, "originator": "codex_cli_rs" }
    }))
}

fn turn_context(ts: &str, cwd: &serde_json::Value) -> String {
    line(
        &json!({ "timestamp": ts, "type": "turn_context", "payload": { "cwd": cwd, "model": "gpt-5.5" } }),
    )
}

fn record(ts: &str, id: &str) -> String {
    usage_record(ts, id, &usage(10, 0, 1, 0))
}

fn projects(events: &[UsageEvent]) -> Vec<Option<&str>> {
    events
        .iter()
        .map(|event| event.project.as_deref())
        .collect()
}

#[test]
fn fixtures_attach_the_session_cwd() {
    let records = parse(RECORDS);
    assert_eq!(projects(&records), [Some("/home/user/project"); 3]);
    let legacy = parse(LEGACY);
    assert!(!legacy.is_empty());
    assert!(
        legacy
            .iter()
            .all(|event| event.project.as_deref() == Some("/home/user/project"))
    );
}

#[test]
fn the_latest_string_cwd_wins() {
    let rows = [
        (json!(API), Some(API)),
        (json!(""), Some(APP)),
        (json!(null), Some(APP)),
        (json!(5), Some(APP)),
        (json!({ "path": API }), Some(APP)),
    ];
    for (cwd, expected) in rows {
        let content = [
            session_meta(&json!(APP)),
            turn_context("2026-09-22T10:00:01Z", &json!(APP)),
            record("2026-09-22T10:00:02Z", "r1"),
            turn_context("2026-09-22T10:00:03Z", &cwd),
            record("2026-09-22T10:00:04Z", "r2"),
        ]
        .join("\n");
        assert_eq!(projects(&parse(&content)), [Some(APP), expected], "{cwd}");
    }
}

#[test]
fn logs_without_cwd_have_no_project() {
    let content = [
        line(&json!({ "timestamp": "2026-09-22T10:00:01Z", "type": "turn_context", "payload": { "model": "gpt-5.5" } })),
        record("2026-09-22T10:00:02Z", "r1"),
    ]
    .join("\n");
    assert_eq!(projects(&parse(&content)), [None]);
}

#[test]
fn a_record_waiting_for_its_model_gets_the_project_stated_with_it() {
    let content = [
        record("2026-09-22T10:00:00Z", "r1"),
        turn_context("2026-09-22T10:00:01Z", &json!(APP)),
    ]
    .join("\n");
    let events = parse(&content);
    assert_eq!(projects(&events), [Some(APP)]);
    assert_eq!(events[0].model, "gpt-5.5");
}

#[test]
fn cursor_round_trip_keeps_the_project() {
    let first = session_meta(&json!(APP));
    let second = [
        turn_context("2026-09-22T10:00:01Z", &json!(null)),
        record("2026-09-22T10:00:02Z", "r1"),
    ]
    .join("\n");
    let mut state = ParserState::default();
    assert!(parse_with(&mut state, &first, LONG_AFTER).is_empty());
    let saved = state.to_value().unwrap();
    assert_eq!(saved["project"], json!(APP));
    let mut restored = ParserState::from_value(&saved);
    assert_eq!(restored, state);
    let events = parse_with(&mut restored, &second, LONG_AFTER);
    assert_eq!(projects(&events), [Some(APP)]);
}

#[test]
fn cursors_saved_before_projects_existed_still_load() {
    let pending = token_count_line();
    let mut state = ParserState::default();
    assert!(parse_with(&mut state, &pending, "2026-09-22T10:00:01Z").is_empty());
    let mut saved = state.to_value().unwrap();
    let object = saved.as_object_mut().unwrap();
    assert_eq!(object.remove("project"), Some(json!(null)));
    for event in object["pending"].as_array_mut().unwrap() {
        assert!(event.get("project").is_none());
    }
    let mut restored = ParserState::from_value(&saved);
    assert_eq!(restored, state);
    let events = parse_with(&mut restored, "", LONG_AFTER);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].project, None);
}

fn token_count_line() -> String {
    let last = usage(90, 0, 9, 0);
    line(&json!({
        "timestamp": "2026-09-22T10:00:00Z",
        "type": "event_msg",
        "payload": { "type": "token_count", "info": { "total_token_usage": last, "last_token_usage": last } }
    }))
}
