use serde_json::json;

use super::*;

const SESSION: &str = include_str!("fixtures/session.jsonl");

fn session_line(index: usize) -> &'static str {
    SESSION.lines().nth(index).unwrap()
}

fn parsed(index: usize) -> Option<UsageEvent> {
    parse_line(session_line(index))
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
fn missing_request_id_uses_fallback_key() {
    let event = parsed(8).unwrap();
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
    assert_eq!(parse_line(&assistant(&usage, "claude-opus-5-5")), None);
}

#[test]
fn priority_service_tier_is_mapped() {
    let usage = json!({ "input_tokens": 1, "output_tokens": 1, "service_tier": "priority" });
    let event = parse_line(&assistant(&usage, "claude-opus-5-5")).unwrap();
    assert_eq!(event.tier, ServiceTier::Priority);
}

#[test]
fn empty_model_and_bad_timestamps_are_skipped() {
    let usage = json!({ "input_tokens": 1, "output_tokens": 1 });
    assert_eq!(parse_line(&assistant(&usage, "")), None);
    let line = assistant(&usage, "m").replace("2026-09-20T10:00:00Z", "yesterday");
    assert_eq!(parse_line(&line), None);
}

#[test]
fn garbage_lines_are_skipped() {
    assert_eq!(parse_line("{\"type\":\"assistant\""), None);
    assert_eq!(parse_line("[]"), None);
    assert_eq!(parse_line("null"), None);
}
