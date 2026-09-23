use std::collections::BTreeMap;

use headroom_core::event::{EventKey, ServiceTier, UsageEvent};
use headroom_core::tokens::TokenCounts;
use headroom_core::units::{MicroUsd, Tokens};
use jiff::Timestamp;
use serde::Deserialize;
use serde_json::{Number, Value};

const TURN_COMPLETED: &str = "turn_completed";
const TICKS_PER_MICRO_USD: i64 = 10_000;
const LARGEST_EXACT_F64_INTEGER: f64 = 9_007_199_254_740_992.0;

#[derive(Deserialize)]
struct Line {
    timestamp: Option<Value>,
    params: Option<Params>,
    update: Option<Update>,
    #[serde(rename = "_meta")]
    meta: Option<Meta>,
}

#[derive(Deserialize)]
struct Params {
    update: Option<Update>,
    #[serde(rename = "_meta")]
    meta: Option<Meta>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Update {
    session_update: Option<String>,
    usage: Option<TurnUsage>,
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Meta {
    event_id: Option<String>,
    agent_timestamp_ms: Option<Number>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TurnUsage {
    cost_usd_ticks: Option<Number>,
    #[serde(default)]
    model_usage: BTreeMap<String, ModelUsage>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ModelUsage {
    input_tokens: Number,
    cached_read_tokens: Option<Number>,
    cache_creation_tokens: Option<Number>,
    output_tokens: Option<Number>,
    reasoning_tokens: Option<Number>,
    cost_usd_ticks: Option<Number>,
}

struct Turn {
    at: Timestamp,
    event_id: Option<String>,
    usage: TurnUsage,
}

pub(super) fn parse_line(line: &str) -> Vec<UsageEvent> {
    if !line.contains(TURN_COMPLETED) {
        return Vec::new();
    }
    let parsed = serde_json::from_str::<Line>(line)
        .inspect_err(|error| tracing::warn!(%error, "skipping an unreadable grok turn"));
    let Some(turn) = parsed.ok().and_then(completed_turn) else {
        return Vec::new();
    };
    let single_model = turn.usage.model_usage.len() == 1;
    turn.usage
        .model_usage
        .iter()
        .filter_map(|(model, usage)| {
            let fallback_ticks = turn.usage.cost_usd_ticks.as_ref().filter(|_| single_model);
            model_event(&turn, model.trim(), usage, fallback_ticks)
        })
        .collect()
}

fn completed_turn(line: Line) -> Option<Turn> {
    let (update, meta) = match line.params {
        Some(params) => (params.update, params.meta),
        None => (line.update, line.meta),
    };
    let update =
        update.filter(|update| update.session_update.as_deref() == Some(TURN_COMPLETED))?;
    let meta = meta.unwrap_or_default();
    Some(Turn {
        at: turn_time(&meta, line.timestamp.as_ref())?,
        event_id: meta
            .event_id
            .map(|id| id.trim().to_owned())
            .filter(|id| !id.is_empty()),
        usage: update.usage?,
    })
}

fn turn_time(meta: &Meta, timestamp: Option<&Value>) -> Option<Timestamp> {
    let from_millis = meta
        .agent_timestamp_ms
        .as_ref()
        .and_then(whole)
        .filter(|millis| *millis > 0)
        .and_then(|millis| Timestamp::from_millisecond(millis).ok());
    from_millis.or_else(|| match timestamp? {
        Value::Number(seconds) => whole(seconds)
            .filter(|seconds| *seconds > 0)
            .and_then(|seconds| Timestamp::from_second(seconds).ok()),
        Value::String(text) => text.trim().parse().ok(),
        _ => None,
    })
}

fn model_event(
    turn: &Turn,
    model: &str,
    usage: &ModelUsage,
    fallback_ticks: Option<&Number>,
) -> Option<UsageEvent> {
    if model.is_empty() {
        return None;
    }
    let Some(tokens) = token_counts(usage) else {
        tracing::warn!(model, "skipping grok usage with invalid token counts");
        return None;
    };
    let ticks = usage.cost_usd_ticks.as_ref().or(fallback_ticks);
    Some(UsageEvent {
        key: event_key(turn, model, &tokens),
        at: turn.at,
        model: model.to_owned(),
        tier: ServiceTier::Standard,
        tokens,
        web_search_requests: 0,
        reported_cost: ticks.and_then(whole).and_then(micro_usd_from_ticks),
    })
}

fn event_key(turn: &Turn, model: &str, tokens: &TokenCounts) -> EventKey {
    match &turn.event_id {
        Some(id) => EventKey(format!("{id}:{model}")),
        None => EventKey::fallback(turn.at, model, tokens),
    }
}

fn token_counts(usage: &ModelUsage) -> Option<TokenCounts> {
    let optional = |value: &Option<Number>| value.as_ref().map_or(Some(0), count);
    let input = count(&usage.input_tokens)?;
    let cache_read = optional(&usage.cached_read_tokens)?.min(input);
    let cache_write = optional(&usage.cache_creation_tokens)?.min(input - cache_read);
    Some(TokenCounts {
        input: Tokens(input - cache_read - cache_write),
        cache_read: Tokens(cache_read),
        cache_write_5m: Tokens(cache_write),
        cache_write_1h: Tokens::ZERO,
        output: Tokens(optional(&usage.output_tokens)?),
        reasoning: Tokens(optional(&usage.reasoning_tokens)?),
    })
}

fn count(number: &Number) -> Option<u64> {
    whole(number).and_then(|value| u64::try_from(value).ok())
}

pub(super) fn micro_usd_from_ticks(ticks: i64) -> Option<MicroUsd> {
    if ticks < 0 {
        return None;
    }
    let whole_micros = ticks / TICKS_PER_MICRO_USD;
    let rest = ticks % TICKS_PER_MICRO_USD;
    Some(MicroUsd(
        whole_micros + i64::from(rest * 2 >= TICKS_PER_MICRO_USD),
    ))
}

fn whole(number: &Number) -> Option<i64> {
    number
        .as_i64()
        .or_else(|| number.as_f64().and_then(exact_integer))
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::float_cmp,
    reason = "an exact whole-number check on a value inside the range f64 represents exactly"
)]
fn exact_integer(value: f64) -> Option<i64> {
    let is_whole = value.is_finite() && value.trunc() == value;
    (is_whole && value.abs() <= LARGEST_EXACT_F64_INTEGER).then_some(value as i64)
}

#[cfg(test)]
#[path = "turn_tests.rs"]
mod tests;
