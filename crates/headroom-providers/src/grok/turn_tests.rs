use super::*;

const UPDATES: &str = include_str!("fixtures/updates.jsonl");
const FORKED: &str = include_str!("fixtures/forked.jsonl");

fn events(text: &str) -> Vec<UsageEvent> {
    text.lines().flat_map(parse_line).collect()
}

fn ts(text: &str) -> Timestamp {
    text.parse().unwrap()
}

#[test]
fn completed_turns_become_one_event_per_model() {
    let events = events(UPDATES);
    assert_eq!(events.len(), 2);
    let first = &events[0];
    assert_eq!(
        first.key.0,
        "01a0f000-0000-7000-8000-000000000001-3345:grok-4.6-build"
    );
    assert_eq!(first.model, "grok-4.6-build");
    assert_eq!(
        first.at,
        Timestamp::from_millisecond(1_789_565_936_743).unwrap()
    );
    assert_eq!(first.tier, ServiceTier::Standard);
    assert_eq!(
        first.tokens,
        TokenCounts {
            input: Tokens(7_079_304 - 6_731_392),
            cache_read: Tokens(6_731_392),
            cache_write_5m: Tokens::ZERO,
            cache_write_1h: Tokens::ZERO,
            output: Tokens(78_284),
            reasoning: Tokens(69_716),
        }
    );
    assert_eq!(first.tokens.total(), Tokens(7_157_588));
    assert_eq!(first.reported_cost, Some(MicroUsd(1_540_616)));
}

#[test]
fn half_micro_dollars_round_up() {
    assert_eq!(events(UPDATES)[1].reported_cost, Some(MicroUsd(22_500)));
}

#[test]
fn tick_conversion_is_exact_integer_math() {
    let cases = [
        (0, Some(0)),
        (4_999, Some(0)),
        (5_000, Some(1)),
        (9_999, Some(1)),
        (10_000, Some(1)),
        (15_406_161_600, Some(1_540_616)),
        (15_406_164_999, Some(1_540_616)),
        (15_406_165_000, Some(1_540_617)),
        (i64::MAX, Some(922_337_203_685_478)),
        (-1, None),
    ];
    for (ticks, micros) in cases {
        assert_eq!(micro_usd_from_ticks(ticks), micros.map(MicroUsd), "{ticks}");
    }
}

#[test]
fn cache_writes_come_out_of_input_and_cost_needs_its_own_model() {
    let events = events(FORKED);
    let multi: Vec<_> = events.iter().skip(1).collect();
    assert_eq!(multi[0].model, "grok-4.7-build");
    assert_eq!(
        multi[0].tokens.input,
        Tokens(12_300_000 - 11_890_000 - 1_024)
    );
    assert_eq!(multi[0].tokens.cache_write_5m, Tokens(1_024));
    assert_eq!(multi[0].reported_cost, Some(MicroUsd(10_447_922)));
    assert_eq!(multi[1].model, "grok-code-fast");
    assert_eq!(multi[1].tokens.input, Tokens(18_493 - 2_736));
    assert_eq!(multi[1].reported_cost, None);
    assert_eq!(
        multi[1].key.0,
        "01a0f000-0000-7000-8000-000000000002-4401:grok-code-fast"
    );
}

#[test]
fn non_turn_lines_and_cancelled_turns_are_ignored() {
    let lines: Vec<&str> = UPDATES.lines().collect();
    assert!(parse_line(lines[0]).is_empty());
    assert!(parse_line(lines[2]).is_empty());
    assert!(parse_line("{not json turn_completed").is_empty());
    assert!(parse_line("").is_empty());
}

#[test]
fn timestamps_fall_back_to_seconds_or_text() {
    let seconds = r#"{"timestamp":1789565936,"update":{"sessionUpdate":"turn_completed","usage":{"modelUsage":{"m":{"inputTokens":1}}}}}"#;
    let text = r#"{"timestamp":"2026-09-23T10:00:00Z","update":{"sessionUpdate":"turn_completed","usage":{"modelUsage":{"m":{"inputTokens":1}}}}}"#;
    let none = r#"{"update":{"sessionUpdate":"turn_completed","usage":{"modelUsage":{"m":{"inputTokens":1}}}}}"#;
    assert_eq!(
        parse_line(seconds)[0].at,
        Timestamp::from_second(1_789_565_936).unwrap()
    );
    assert_eq!(parse_line(text)[0].at, ts("2026-09-23T10:00:00Z"));
    assert!(parse_line(none).is_empty());
}

#[test]
fn turns_without_an_event_id_get_a_fallback_key() {
    let line = r#"{"timestamp":1789565936,"params":{"update":{"sessionUpdate":"turn_completed","usage":{"costUsdTicks":20000,"modelUsage":{"m":{"inputTokens":5,"outputTokens":2}}}}}}"#;
    let events = parse_line(line);
    assert!(events[0].key.is_fallback());
    assert_eq!(events[0].reported_cost, Some(MicroUsd(2)));
    assert_eq!(parse_line(line), events);
}

#[test]
fn invalid_counts_and_costs_are_not_guessed() {
    let bad_counts = r#"{"timestamp":1,"update":{"sessionUpdate":"turn_completed","usage":{"modelUsage":{"m":{"inputTokens":-5},"n":{"inputTokens":3,"outputTokens":1.5}," ":{"inputTokens":1}}}}}"#;
    assert!(parse_line(bad_counts).is_empty());
    let odd_cost = r#"{"timestamp":1,"update":{"sessionUpdate":"turn_completed","usage":{"modelUsage":{"m":{"inputTokens":3,"cachedReadTokens":9,"costUsdTicks":12.5}}}}}"#;
    let events = parse_line(odd_cost);
    assert_eq!(events[0].tokens.cache_read, Tokens(3));
    assert_eq!(events[0].tokens.input, Tokens::ZERO);
    assert_eq!(events[0].reported_cost, None);
    let negative = odd_cost.replace("12.5", "-10000");
    assert_eq!(parse_line(&negative)[0].reported_cost, None);
    let float_whole = odd_cost.replace("12.5", "1.5e10");
    assert_eq!(
        parse_line(&float_whole)[0].reported_cost,
        Some(MicroUsd(1_500_000))
    );
}
