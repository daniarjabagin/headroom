use super::*;
use crate::event::EventKey;

struct FlatPrices;

impl PriceBook for FlatPrices {
    fn cost(
        &self,
        model: &str,
        tier: ServiceTier,
        tokens: &TokenCounts,
        web_search: u32,
    ) -> Option<MicroUsd> {
        let rate = match (model, tier) {
            ("unknown", _) => return None,
            (_, ServiceTier::Priority) => 2,
            _ => 1,
        };
        let base = i64::try_from(tokens.total().0).ok()? * rate;
        Some(MicroUsd(base + i64::from(web_search) * 10_000))
    }
}

fn ts(text: &str) -> Timestamp {
    text.parse().unwrap()
}

fn event(key: &str, at: &str, model: &str, output: u64) -> UsageEvent {
    UsageEvent {
        key: EventKey(key.into()),
        at: ts(at),
        model: model.into(),
        tier: ServiceTier::Standard,
        tokens: TokenCounts {
            input: Tokens(100),
            output: Tokens(output),
            reasoning: Tokens(output / 2),
            ..TokenCounts::default()
        },
        web_search_requests: 0,
    }
}

fn tz(name: &str) -> TimeZone {
    TimeZone::get(name).unwrap()
}

fn now() -> Timestamp {
    ts("2026-09-23T10:00:00Z")
}

fn run(events: &[UsageEvent], zone: &TimeZone) -> UsageSummary {
    aggregate(events, &FlatPrices, zone, now())
}

fn date(text: &str) -> Date {
    text.parse().unwrap()
}

#[test]
fn empty_input_gives_empty_summary() {
    assert_eq!(run(&[], &TimeZone::UTC), UsageSummary::default());
}

#[test]
fn day_bucketing_follows_eastern_time_zone() {
    let events = [event("a", "2026-09-22T20:00:00Z", "gpt-5.5", 0)];
    let summary = run(&events, &tz("Asia/Almaty"));
    assert_eq!(summary.today.tokens.total(), Tokens(100));
    assert_eq!(summary.yesterday, UsageTotals::default());
    assert_eq!(summary.daily[0].0, date("2026-09-23"));
}

#[test]
fn day_bucketing_follows_western_time_zone() {
    let events = [event("a", "2026-09-22T20:00:00Z", "gpt-5.5", 0)];
    let summary = run(&events, &tz("America/Los_Angeles"));
    assert_eq!(summary.today, UsageTotals::default());
    assert_eq!(summary.yesterday.tokens.total(), Tokens(100));
    assert_eq!(summary.daily[0].0, date("2026-09-22"));
}

#[test]
fn today_starts_at_local_midnight() {
    let zone = tz("America/Los_Angeles");
    let events = [
        event("before", "2026-09-23T06:59:59Z", "m", 0),
        event("after", "2026-09-23T07:00:00Z", "m", 0),
    ];
    let summary = aggregate(&events, &FlatPrices, &zone, ts("2026-09-23T20:00:00Z"));
    assert_eq!(summary.today.tokens.input, Tokens(100));
    assert_eq!(summary.yesterday.tokens.input, Tokens(100));
}

#[test]
fn thirty_day_window_includes_today_and_29_previous_days() {
    let events = [
        event("too-old", "2026-08-24T23:59:59Z", "m", 0),
        event("first", "2026-08-25T00:00:00Z", "m", 0),
        event("last", "2026-09-23T23:59:59Z", "m", 0),
        event("future", "2026-09-24T00:00:00Z", "m", 0),
    ];
    let summary = run(&events, &TimeZone::UTC);
    assert_eq!(summary.last_30_days.tokens.input, Tokens(200));
    let days: Vec<Date> = summary.daily.iter().map(|(day, _)| *day).collect();
    assert_eq!(days, [date("2026-08-25"), date("2026-09-23")]);
}

#[test]
fn daily_is_sorted_and_only_has_active_days() {
    let events = [
        event("c", "2026-09-23T01:00:00Z", "m", 0),
        event("a", "2026-09-20T01:00:00Z", "m", 0),
        event("b", "2026-09-20T02:00:00Z", "m", 0),
    ];
    let summary = run(&events, &TimeZone::UTC);
    assert_eq!(summary.daily.len(), 2);
    assert_eq!(summary.daily[0].0, date("2026-09-20"));
    assert_eq!(summary.daily[0].1.tokens.input, Tokens(200));
}

#[test]
fn unpriced_models_keep_tokens_but_not_cost() {
    let events = [
        event("a", "2026-09-23T01:00:00Z", "gpt-5.5", 50),
        event("b", "2026-09-23T02:00:00Z", "unknown", 20),
    ];
    let summary = run(&events, &TimeZone::UTC);
    assert_eq!(summary.today.tokens.total(), Tokens(270));
    assert_eq!(summary.today.cost, MicroUsd(150));
    assert_eq!(summary.today.unpriced_tokens, Tokens(120));
    assert_eq!(
        summary.today.unpriced_models,
        BTreeSet::from(["unknown".to_string()])
    );
    assert!(summary.today.is_partial());
    assert!(summary.last_30_days.is_partial());
}

#[test]
fn fully_priced_totals_are_not_partial() {
    let summary = run(
        &[event("a", "2026-09-23T01:00:00Z", "m", 1)],
        &TimeZone::UTC,
    );
    assert!(!summary.today.is_partial());
    assert_eq!(summary.today.unpriced_tokens, Tokens::ZERO);
}

#[test]
fn tier_and_web_search_reach_the_price_book() {
    let mut priority = event("a", "2026-09-23T01:00:00Z", "m", 0);
    priority.tier = ServiceTier::Priority;
    priority.web_search_requests = 2;
    let summary = run(&[priority], &TimeZone::UTC);
    assert_eq!(summary.today.cost, MicroUsd(200 + 20_000));
}

#[test]
fn reasoning_is_not_double_counted_in_totals() {
    let summary = run(
        &[event("a", "2026-09-23T01:00:00Z", "m", 40)],
        &TimeZone::UTC,
    );
    assert_eq!(summary.today.tokens.reasoning, Tokens(20));
    assert_eq!(summary.today.tokens.total(), Tokens(140));
    assert_eq!(summary.today.cost, MicroUsd(140));
}

#[test]
fn duplicate_keys_are_summed_because_dedup_happens_upstream() {
    let events = [
        event("same", "2026-09-23T01:00:00Z", "m", 0),
        event("same", "2026-09-23T01:00:00Z", "m", 0),
    ];
    assert_eq!(run(&events, &TimeZone::UTC).today.tokens.input, Tokens(200));
}

#[test]
fn models_are_sorted_by_cost_then_tokens_then_name() {
    let events = [
        event("a", "2026-09-23T01:00:00Z", "unknown", 900),
        event("b", "2026-09-23T01:00:00Z", "cheap", 0),
        event("c", "2026-09-23T01:00:00Z", "pricey", 500),
        event("d", "2026-09-23T01:00:00Z", "also-cheap", 0),
    ];
    let summary = run(&events, &TimeZone::UTC);
    let names: Vec<&str> = summary.models.iter().map(|m| m.model.as_str()).collect();
    assert_eq!(names, ["pricey", "also-cheap", "cheap", "unknown"]);
    assert!(summary.models[3].totals.is_partial());
}

#[test]
fn models_only_cover_the_thirty_day_window() {
    let events = [event("old", "2026-07-01T00:00:00Z", "ancient", 0)];
    assert!(run(&events, &TimeZone::UTC).models.is_empty());
}
