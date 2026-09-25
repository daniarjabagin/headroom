use serde_json::json;

use super::*;

const SESSION: &str = include_str!("fixtures/session.jsonl");

fn session_line(index: usize) -> &'static str {
    SESSION.lines().nth(index).unwrap()
}

fn single(line: &str) -> Option<UsageEvent> {
    let mut events = parse_line(line);
    assert!(
        events.len() <= 1,
        "expected at most one event, got {events:?}"
    );
    events.pop()
}

fn parsed(index: usize) -> Option<UsageEvent> {
    single(session_line(index))
}

#[test]
fn non_assistant_lines_are_ignored() {
    assert_eq!(parsed(0), None);
    assert_eq!(parsed(4), None);
    assert_eq!(parsed(6), None);
}

#[test]
fn streaming_block_carries_partial_usage_and_key() {
    let event = parsed(1).unwrap();
    assert_eq!(event.key, EventKey("msg_fake_A:req_fake_A".into()));
    assert_eq!(
        event.at,
        "2026-09-20T10:00:05.1Z".parse::<Timestamp>().unwrap()
    );
    assert_eq!(event.model, "claude-opus-5-5");
    assert_eq!(event.tier, ServiceTier::Standard);
    assert_eq!(
        event.tokens,
        TokenCounts {
            input: Tokens(3),
            cache_read: Tokens(15_000),
            cache_write_5m: Tokens(0),
            cache_write_1h: Tokens(2_000),
            output: Tokens(12),
            reasoning: Tokens(10),
        }
    );
}

#[test]
fn nested_null_fields_in_tool_input_do_not_drop_the_line() {
    let event = parsed(3).unwrap();
    assert_eq!(event.tokens.output, Tokens(332));
    assert_eq!(event.key, EventKey("msg_fake_A:req_fake_A".into()));
}

#[test]
fn five_minute_cache_and_web_search_are_read() {
    let event = parsed(5).unwrap();
    assert_eq!(event.tokens.cache_write_5m, Tokens(700));
    assert_eq!(event.tokens.cache_write_1h, Tokens(0));
    assert_eq!(event.tokens.reasoning, Tokens(0));
    assert_eq!(event.web_search_requests, 2);
    assert_eq!(event.tokens.total(), Tokens(5 + 17_000 + 700 + 40));
}

#[test]
fn synthetic_api_error_is_skipped() {
    assert_eq!(parsed(7), None);
}

#[test]
fn legacy_cache_total_counts_as_five_minute_write() {
    let event = parsed(8).unwrap();
    assert_eq!(event.tokens.cache_write_5m, Tokens(300));
    assert_eq!(event.tier, ServiceTier::Fast);
}

#[test]
fn missing_request_id_keys_on_message_id() {
    let event = parsed(8).unwrap();
    assert_eq!(event.key, EventKey("msg_fake_D".into()));
}

#[test]
fn missing_message_and_request_ids_use_fallback_key() {
    let line = session_line(8).replace("\"id\":\"msg_fake_D\",", "");
    let event = single(&line).unwrap();
    assert!(event.key.is_fallback());
    assert_eq!(
        event.key,
        EventKey::fallback(event.at, &event.model, &event.tokens)
    );
}

#[test]
fn non_integer_token_counts_drop_the_line() {
    assert_eq!(parsed(9), None);
}

fn assistant(usage: &serde_json::Value, model: &str) -> String {
    json!({
        "type": "assistant",
        "timestamp": "2026-09-20T10:00:00Z",
        "requestId": "req_x",
        "isApiErrorMessage": true,
        "message": { "id": "msg_x", "model": model, "usage": usage }
    })
    .to_string()
}

#[test]
fn zero_usage_error_lines_are_skipped_even_with_real_model() {
    let usage = json!({ "input_tokens": 0, "output_tokens": 0 });
    assert!(parse_line(&assistant(&usage, "claude-opus-5-5")).is_empty());
}

#[test]
fn priority_service_tier_is_mapped() {
    let usage = json!({ "input_tokens": 1, "output_tokens": 1, "service_tier": "priority" });
    let event = single(&assistant(&usage, "claude-opus-5-5")).unwrap();
    assert_eq!(event.tier, ServiceTier::Priority);
}

#[test]
fn empty_model_and_bad_timestamps_are_skipped() {
    let usage = json!({ "input_tokens": 1, "output_tokens": 1 });
    assert!(parse_line(&assistant(&usage, "")).is_empty());
    let line = assistant(&usage, "m").replace("2026-09-20T10:00:00Z", "yesterday");
    assert!(parse_line(&line).is_empty());
}

#[test]
fn records_beyond_the_storable_range_are_skipped() {
    let usage = json!({ "input_tokens": 1, "output_tokens": 1 });
    let far_future = assistant(&usage, "m").replace("2026-09-20", "2300-01-01");
    assert!(parse_line(&far_future).is_empty());
    let huge = json!({ "input_tokens": u64::MAX, "output_tokens": 1 });
    assert!(parse_line(&assistant(&huge, "m")).is_empty());
    let overflowing = json!({ "input_tokens": i64::MAX, "output_tokens": 1 });
    assert!(parse_line(&assistant(&overflowing, "m")).is_empty());
    let largest = json!({ "input_tokens": i64::MAX, "output_tokens": 0 });
    assert_eq!(parse_line(&assistant(&largest, "m")).len(), 1);
}

#[test]
fn garbage_lines_are_skipped() {
    assert!(parse_line("{\"type\":\"assistant\"").is_empty());
    assert!(parse_line("[]").is_empty());
    assert!(parse_line("null").is_empty());
}

const ITERATIONS: &str = include_str!("fixtures/iterations.jsonl");

fn iteration_line(index: usize) -> &'static str {
    ITERATIONS.lines().nth(index).unwrap()
}

fn one_hour_write(input: u64, cache_write_1h: u64, output: u64) -> TokenCounts {
    TokenCounts {
        input: Tokens(input),
        cache_write_1h: Tokens(cache_write_1h),
        output: Tokens(output),
        ..TokenCounts::default()
    }
}

#[test]
fn fallback_message_emits_the_earlier_iteration_with_its_own_model() {
    let events = parse_line(iteration_line(0));
    let summary: Vec<(&str, &str, TokenCounts)> = events
        .iter()
        .map(|e| (e.key.0.as_str(), e.model.as_str(), e.tokens))
        .collect();
    assert_eq!(
        summary,
        [
            (
                "msg_fake_F:req_fake_F",
                "claude-opus-4-8",
                one_hour_write(2, 3_696, 5)
            ),
            (
                "msg_fake_F:req_fake_F:iter:0",
                "claude-fable-5-1",
                one_hour_write(47, 3_697, 0)
            ),
        ]
    );
    assert_eq!(events[1].at, events[0].at);
    assert_eq!(events[1].web_search_requests, 0);
}

#[test]
fn single_iteration_is_already_the_top_level_usage() {
    let event = single(iteration_line(2)).unwrap();
    assert_eq!(event.key, EventKey("msg_fake_G:req_fake_G".into()));
    assert_eq!(event.tokens.total(), Tokens(2 + 24_000 + 1_600 + 287));
}

fn with_iterations(iterations: &serde_json::Value, request_id: bool) -> String {
    let mut line = json!({
        "type": "assistant",
        "timestamp": "2026-09-20T10:00:00Z",
        "message": {
            "id": "msg_x",
            "model": "claude-opus-4-8",
            "usage": { "input_tokens": 2, "output_tokens": 5, "iterations": iterations }
        }
    });
    if request_id {
        line["requestId"] = json!("req_x");
    }
    line.to_string()
}

fn iteration(input: u64, model: Option<&str>) -> serde_json::Value {
    json!({ "type": "message", "model": model, "input_tokens": input, "output_tokens": 0 })
}

fn keys_and_models(line: &str) -> Vec<(String, String)> {
    parse_line(line)
        .into_iter()
        .map(|e| (e.key.0, e.model))
        .collect()
}

#[test]
fn iteration_without_model_uses_the_message_model() {
    let line = with_iterations(&json!([iteration(9, None), iteration(2, None)]), true);
    assert_eq!(
        keys_and_models(&line),
        [
            ("msg_x:req_x".to_owned(), "claude-opus-4-8".to_owned()),
            (
                "msg_x:req_x:iter:0".to_owned(),
                "claude-opus-4-8".to_owned()
            ),
        ]
    );
}

#[test]
fn iteration_without_request_id_keys_on_message_id() {
    let line = with_iterations(
        &json!([iteration(9, Some("claude-fable-5-1")), iteration(2, None)]),
        false,
    );
    assert_eq!(
        keys_and_models(&line),
        [
            ("msg_x".to_owned(), "claude-opus-4-8".to_owned()),
            ("msg_x:iter:0".to_owned(), "claude-fable-5-1".to_owned()),
        ]
    );
}

#[test]
fn every_iteration_before_the_last_is_emitted_by_index() {
    let line = with_iterations(
        &json!([
            iteration(7, Some("a")),
            iteration(0, Some("zero")),
            iteration(8, Some("b")),
            iteration(2, None)
        ]),
        true,
    );
    let keys: Vec<String> = keys_and_models(&line).into_iter().map(|(k, _)| k).collect();
    assert_eq!(
        keys,
        ["msg_x:req_x", "msg_x:req_x:iter:0", "msg_x:req_x:iter:2"]
    );
}

#[test]
fn malformed_iteration_keeps_the_rest_of_the_line() {
    let line = with_iterations(
        &json!([{ "type": "message", "input_tokens": "9" }, iteration(3, Some("a")), iteration(2, None)]),
        true,
    );
    let keys: Vec<String> = keys_and_models(&line).into_iter().map(|(k, _)| k).collect();
    assert_eq!(keys, ["msg_x:req_x", "msg_x:req_x:iter:1"]);
}

#[test]
fn null_or_empty_iterations_add_nothing() {
    assert_eq!(parse_line(&with_iterations(&json!(null), true)).len(), 1);
    assert_eq!(parse_line(&with_iterations(&json!([]), true)).len(), 1);
}

#[test]
fn session_cwd_becomes_the_project() {
    assert_eq!(
        parsed(1).unwrap().project.as_deref(),
        Some("/home/user/project")
    );
    assert_eq!(parsed(5).unwrap().project, None);
}

fn with_cwd(cwd: &serde_json::Value) -> String {
    let iterations = json!([iteration(9, Some("claude-fable-5-1")), iteration(2, None)]);
    let mut line: serde_json::Value =
        serde_json::from_str(&with_iterations(&iterations, true)).unwrap();
    line["cwd"] = cwd.clone();
    line.to_string()
}

#[test]
fn every_event_of_a_line_carries_its_cwd() {
    let rows = [
        (json!("/home/user/work/app"), Some("/home/user/work/app")),
        (json!(""), None),
        (json!(null), None),
        (json!(5), None),
        (json!({ "path": "/home/user/work/app" }), None),
    ];
    for (cwd, expected) in rows {
        let events = parse_line(&with_cwd(&cwd));
        assert_eq!(events.len(), 2, "{cwd}");
        for event in events {
            assert_eq!(event.project.as_deref(), expected, "{cwd}");
        }
    }
}
