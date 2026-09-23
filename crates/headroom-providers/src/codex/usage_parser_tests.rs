use serde_json::json;

use super::super::test_support::at;
use super::*;

const RECORDS: &str = include_str!("fixtures/rollout_records.jsonl");
const LEGACY: &str = include_str!("fixtures/rollout_legacy.jsonl");
const CUMULATIVE: &str = include_str!("fixtures/rollout_cumulative.jsonl");
const GUARDIAN: &str = include_str!("fixtures/rollout_guardian.jsonl");
const GUARDIAN_AT: &str = "2026-09-12T14:01:34.836Z";
const LONG_AFTER: &str = "2026-12-01T00:00:00Z";

fn parse_with(state: &mut ParserState, content: &str, now: &str) -> Vec<UsageEvent> {
    let mut events = Vec::new();
    for line in content.lines() {
        state.consume(line, &mut events);
    }
    state.settle(at(now), &mut events);
    events
}

fn parse(content: &str) -> Vec<UsageEvent> {
    parse_with(&mut ParserState::default(), content, LONG_AFTER)
}

fn counts(input: u64, cache_read: u64, output: u64, reasoning: u64) -> TokenCounts {
    TokenCounts {
        input: Tokens(input),
        cache_read: Tokens(cache_read),
        output: Tokens(output),
        reasoning: Tokens(reasoning),
        ..TokenCounts::default()
    }
}

fn line(value: &serde_json::Value) -> String {
    value.to_string()
}

fn usage(input: u64, cached: u64, output: u64, reasoning: u64) -> serde_json::Value {
    json!({
        "input_tokens": input,
        "cached_input_tokens": cached,
        "cache_write_input_tokens": 0,
        "output_tokens": output,
        "reasoning_output_tokens": reasoning,
        "total_tokens": input + output,
    })
}

fn token_count(ts: &str, last: &serde_json::Value, total: &serde_json::Value) -> String {
    line(&json!({
        "timestamp": ts,
        "type": "event_msg",
        "payload": { "type": "token_count", "info": { "total_token_usage": total, "last_token_usage": last } }
    }))
}

fn usage_record(ts: &str, response_id: &str, usage: &serde_json::Value) -> String {
    line(&json!({
        "timestamp": ts,
        "type": "token_usage_record",
        "payload": { "response_id": response_id, "usage": usage }
    }))
}

fn settings(ts: &str, model: &str, tier: &str) -> String {
    line(&json!({
        "timestamp": ts,
        "type": "event_msg",
        "payload": { "type": "thread_settings_applied", "thread_settings": { "model": model, "service_tier": tier } }
    }))
}

fn turn_context(ts: &str, model: &str) -> String {
    line(&json!({ "timestamp": ts, "type": "turn_context", "payload": { "model": model } }))
}

#[test]
fn records_file_counts_each_response_exactly_once() {
    let events = parse(RECORDS);
    let summary: Vec<_> = events
        .iter()
        .map(|event| {
            (
                event.key.0.as_str(),
                event.model.as_str(),
                event.tier,
                event.tokens,
            )
        })
        .collect();
    assert_eq!(
        summary,
        [
            (
                "resp_fake_0001",
                "gpt-5.5",
                ServiceTier::Priority,
                counts(200, 800, 50, 20)
            ),
            (
                "resp_fake_0002",
                "gpt-5.5",
                ServiceTier::Priority,
                counts(500, 1500, 100, 0)
            ),
            (
                "resp_fake_0003",
                "gpt-5.5-mini",
                ServiceTier::Standard,
                counts(500, 0, 10, 5)
            ),
        ]
    );
    assert_eq!(events[0].at, at("2026-09-22T07:28:48.626Z"));
}

#[test]
fn reasoning_is_not_added_on_top_of_output() {
    let events = parse(RECORDS);
    assert_eq!(events[0].tokens.total(), Tokens(1050));
    assert_eq!(events[2].tokens.total(), Tokens(510));
}

#[test]
fn legacy_token_count_file_uses_last_usage_and_skips_duplicates() {
    let events = parse(LEGACY);
    assert_eq!(events.len(), 2);
    assert!(events.iter().all(|event| event.key.is_fallback()));
    assert!(events.iter().all(|event| event.model == "gpt-5.4"));
    assert_eq!(events[0].tokens, counts(80, 20, 10, 5));
    assert_eq!(events[1].tokens, counts(80, 10, 15, 5));
    assert_eq!(events[1].at, at("2026-07-10T08:02:00.123456Z"));
    assert_eq!(
        events[0].key,
        EventKey::fallback(
            at("2026-07-10T08:01:00.5Z"),
            "gpt-5.4",
            &counts(80, 20, 10, 5)
        )
    );
}

#[test]
fn cumulative_totals_become_deltas_and_duplicates_are_skipped() {
    let events = parse(CUMULATIVE);
    let tokens: Vec<_> = events.iter().map(|event| event.tokens).collect();
    assert_eq!(tokens, [counts(80, 20, 10, 5), counts(110, 40, 20, 0)]);
    assert!(events.iter().all(|event| event.model == "unknown"));
}

#[test]
fn recent_token_counts_wait_for_a_possible_record() {
    let mut state = ParserState::default();
    let early = parse_with(&mut state, LEGACY, "2026-07-10T08:16:30Z");
    assert_eq!(early.len(), 1);
    let later = parse_with(&mut state, "", "2026-07-10T08:17:00.123456Z");
    assert_eq!(later.len(), 1);
    assert_eq!(later[0].tokens, counts(80, 10, 15, 5));
}

#[test]
fn token_count_written_before_its_record_is_not_double_counted() {
    let content = [
        turn_context("2026-09-22T10:00:00Z", "gpt-5.5"),
        token_count(
            "2026-09-22T10:00:01.000Z",
            &usage(100, 40, 7, 3),
            &usage(100, 40, 7, 3),
        ),
        usage_record("2026-09-22T10:00:01.001Z", "resp_a", &usage(100, 40, 7, 3)),
        token_count(
            "2026-09-22T10:01:00.000Z",
            &usage(50, 0, 5, 0),
            &usage(150, 40, 12, 3),
        ),
        usage_record("2026-09-22T10:01:00.001Z", "resp_b", &usage(50, 0, 5, 0)),
    ]
    .join("\n");
    let events = parse(&content);
    let keys: Vec<_> = events.iter().map(|event| event.key.0.as_str()).collect();
    assert_eq!(keys, ["resp_a", "resp_b"]);
}

#[test]
fn unpaired_legacy_usage_before_the_first_record_is_kept() {
    let content = [
        turn_context("2026-09-22T10:00:00Z", "gpt-5.5"),
        token_count(
            "2026-09-22T10:00:01Z",
            &usage(90, 0, 9, 0),
            &usage(90, 0, 9, 0),
        ),
        usage_record("2026-09-22T10:05:00Z", "resp_a", &usage(100, 40, 7, 3)),
        token_count(
            "2026-09-22T10:05:00.001Z",
            &usage(100, 40, 7, 3),
            &usage(190, 40, 16, 3),
        ),
    ]
    .join("\n");
    let events = parse(&content);
    let summary: Vec<_> = events
        .iter()
        .map(|event| (event.key.is_fallback(), event.tokens))
        .collect();
    assert_eq!(
        summary,
        [(false, counts(60, 40, 7, 3)), (true, counts(90, 0, 9, 0))]
    );
}

#[test]
fn most_recent_model_source_wins() {
    let content = [
        turn_context("2026-09-22T10:00:00Z", "gpt-5.4"),
        settings("2026-09-22T10:00:01Z", "gpt-5.5", "fast"),
        usage_record("2026-09-22T10:00:02Z", "r1", &usage(10, 0, 1, 0)),
        turn_context("2026-09-22T10:00:03Z", "gpt-5.5-codex"),
        usage_record("2026-09-22T10:00:04Z", "r2", &usage(10, 0, 1, 0)),
        settings("2026-09-22T10:00:05Z", " ", "default"),
        usage_record("2026-09-22T10:00:06Z", "r3", &usage(10, 0, 1, 0)),
    ]
    .join("\n");
    let events = parse(&content);
    let summary: Vec<_> = events
        .iter()
        .map(|event| (event.model.as_str(), event.tier))
        .collect();
    assert_eq!(
        summary,
        [
            ("gpt-5.5", ServiceTier::Fast),
            ("gpt-5.5-codex", ServiceTier::Fast),
            ("gpt-5.5-codex", ServiceTier::Standard),
        ]
    );
}

#[test]
fn service_tier_mapping() {
    assert_eq!(tier(Some("priority")), ServiceTier::Priority);
    assert_eq!(tier(Some("Fast")), ServiceTier::Fast);
    assert_eq!(tier(Some("default")), ServiceTier::Standard);
    assert_eq!(tier(Some("flex")), ServiceTier::Standard);
    assert_eq!(tier(None), ServiceTier::Standard);
}

#[test]
fn cached_tokens_are_subtracted_from_input_saturating() {
    let content = [
        usage_record("2026-09-22T10:00:00Z", "r1", &usage(1000, 999, 5, 0)),
        usage_record("2026-09-22T10:00:01Z", "r2", &usage(10, 20, 5, 0)),
    ]
    .join("\n");
    let events = parse(&content);
    assert_eq!(events[0].tokens, counts(1, 999, 5, 0));
    assert_eq!(events[1].tokens, counts(0, 20, 5, 0));
}

#[test]
fn cache_writes_map_to_five_minute_cache() {
    let mut raw = usage(100, 10, 5, 0);
    raw["cache_write_input_tokens"] = json!(7);
    let events = parse(&usage_record("2026-09-22T10:00:00Z", "r1", &raw));
    assert_eq!(events[0].tokens.cache_write_5m, Tokens(7));
    assert_eq!(events[0].tokens.input, Tokens(90));
}

#[test]
fn record_without_response_id_gets_fallback_key() {
    let events = parse(&usage_record(
        "2026-09-22T10:00:00Z",
        " ",
        &usage(10, 0, 1, 0),
    ));
    assert!(events[0].key.is_fallback());
    assert_eq!(events[0].model, "unknown");
}

#[test]
fn state_survives_serialization_mid_file() {
    let lines: Vec<&str> = RECORDS.lines().collect();
    let (first, second) = lines.split_at(8);
    let mut state = ParserState::default();
    let mut events = parse_with(&mut state, &first.join("\n"), LONG_AFTER);
    let mut restored = ParserState::from_value(&state.to_value().unwrap());
    assert_eq!(restored, state);
    events.extend(parse_with(&mut restored, &second.join("\n"), LONG_AFTER));
    assert_eq!(events, parse(RECORDS));
}

#[test]
fn invalid_state_and_garbage_lines_are_tolerated() {
    assert_eq!(
        ParserState::from_value(&json!({"model": 5})),
        ParserState::default()
    );
    assert_eq!(
        ParserState::from_value(&serde_json::Value::Null),
        ParserState::default()
    );
    let content = "not json token_count\n{\"type\":\"token_usage_record\",\"payload\":7}\n";
    assert!(parse(content).is_empty());
}

#[test]
fn guardian_compaction_record_takes_the_model_stated_later_in_the_file() {
    let events = parse_with(&mut ParserState::default(), GUARDIAN, GUARDIAN_AT);
    let summary: Vec<_> = events
        .iter()
        .map(|event| (event.key.0.as_str(), event.model.as_str(), event.tier))
        .collect();
    assert_eq!(
        summary,
        [
            (
                "resp_fake_guardian_compaction",
                "codex-auto-review",
                ServiceTier::Standard
            ),
            (
                "resp_fake_guardian_review",
                "codex-auto-review",
                ServiceTier::Standard
            ),
        ]
    );
    assert_eq!(events[0].tokens, counts(249_538, 4864, 845, 0));
}

#[test]
fn model_less_record_waits_across_reads_for_the_model() {
    let lines: Vec<&str> = GUARDIAN.lines().collect();
    let (first, second) = lines.split_at(4);
    let mut state = ParserState::default();
    assert!(parse_with(&mut state, &first.join("\n"), "2026-09-12T14:02:00Z").is_empty());
    let mut restored = ParserState::from_value(&state.to_value().unwrap());
    let events = parse_with(&mut restored, &second.join("\n"), "2026-09-12T14:02:00Z");
    assert_eq!(events.len(), 2);
    assert!(
        events
            .iter()
            .all(|event| event.model == "codex-auto-review")
    );
}

#[test]
fn model_less_record_becomes_unknown_after_the_pairing_window() {
    let record = usage_record("2026-09-22T10:00:00Z", "r1", &usage(10, 0, 1, 0));
    let mut state = ParserState::default();
    assert!(parse_with(&mut state, &record, "2026-09-22T10:14:59Z").is_empty());
    let events = parse_with(&mut state, "", "2026-09-22T10:15:00Z");
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].model, "unknown");
    let late = parse_with(
        &mut state,
        &turn_context("2026-09-22T10:20:00Z", "gpt-5.5"),
        LONG_AFTER,
    );
    assert!(late.is_empty());
}

#[test]
fn model_less_token_count_is_rekeyed_once_the_model_is_known() {
    let content = [
        token_count(
            "2026-09-22T10:00:00Z",
            &usage(90, 0, 9, 0),
            &usage(90, 0, 9, 0),
        ),
        settings("2026-09-22T10:00:01Z", "gpt-5.5", "priority"),
    ]
    .join("\n");
    let events = parse(&content);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].model, "gpt-5.5");
    assert_eq!(events[0].tier, ServiceTier::Priority);
    assert_eq!(
        events[0].key,
        EventKey::fallback(at("2026-09-22T10:00:00Z"), "gpt-5.5", &counts(90, 0, 9, 0))
    );
}
