use headroom_core::event::{EventKey, ServiceTier, UsageEvent};
use headroom_core::tokens::TokenCounts;
use headroom_core::units::Tokens;
use jiff::Timestamp;
use serde::Deserialize;
use serde_json::Value;

const ASSISTANT: &str = "assistant";
const SYNTHETIC_MODEL: &str = "<synthetic>";

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawLine {
    #[serde(rename = "type")]
    kind: Option<String>,
    timestamp: Option<String>,
    request_id: Option<String>,
    message: Option<RawMessage>,
}

#[derive(Deserialize)]
struct RawMessage {
    id: Option<String>,
    model: Option<String>,
    usage: Option<RawTokenUsage>,
}

#[derive(Deserialize)]
struct RawTokenUsage {
    #[serde(flatten)]
    counts: RawCounts,
    output_tokens_details: Option<RawOutputDetails>,
    server_tool_use: Option<RawServerToolUse>,
    service_tier: Option<String>,
    speed: Option<String>,
    iterations: Option<Vec<Value>>,
}

#[derive(Deserialize)]
struct RawCounts {
    input_tokens: u64,
    output_tokens: u64,
    cache_read_input_tokens: Option<u64>,
    cache_creation_input_tokens: Option<u64>,
    cache_creation: Option<RawCacheCreation>,
}

#[derive(Deserialize)]
struct RawIteration {
    model: Option<String>,
    #[serde(flatten)]
    counts: RawCounts,
}

#[derive(Deserialize)]
struct RawCacheCreation {
    ephemeral_5m_input_tokens: Option<u64>,
    ephemeral_1h_input_tokens: Option<u64>,
}

#[derive(Deserialize)]
struct RawOutputDetails {
    thinking_tokens: Option<u64>,
}

#[derive(Deserialize)]
struct RawServerToolUse {
    web_search_requests: Option<u64>,
}

struct Record {
    at: Timestamp,
    model: String,
    id: Option<String>,
    usage: RawTokenUsage,
}

pub(super) fn parse_line(line: &str) -> Vec<UsageEvent> {
    let Some(record) = parse_record(line) else {
        return Vec::new();
    };
    let mut events: Vec<UsageEvent> = record.top_level_event().into_iter().collect();
    events.extend(record.earlier_iteration_events());
    events.retain(UsageEvent::fits_in_i64);
    events
}

fn parse_record(line: &str) -> Option<Record> {
    let raw: RawLine = serde_json::from_str(line).ok()?;
    if raw.kind.as_deref() != Some(ASSISTANT) {
        return None;
    }
    let message = raw.message?;
    let usage = message.usage?;
    let model = non_empty(message.model).filter(|model| model != SYNTHETIC_MODEL)?;
    let at: Timestamp = raw.timestamp?.parse().ok()?;
    let id = non_empty(message.id).map(|id| match non_empty(raw.request_id) {
        Some(request) => format!("{id}:{request}"),
        None => id,
    });
    Some(Record {
        at,
        model,
        id,
        usage,
    })
}

impl Record {
    fn top_level_event(&self) -> Option<UsageEvent> {
        let tokens = token_counts(&self.usage.counts, thinking_tokens(&self.usage));
        let web_search_requests = web_searches(&self.usage);
        if tokens.total() == Tokens::ZERO && web_search_requests == 0 {
            return None;
        }
        let key = match &self.id {
            Some(id) => EventKey(id.clone()),
            None => EventKey::fallback(self.at, &self.model, &tokens),
        };
        Some(self.event(key, self.model.clone(), tokens, web_search_requests))
    }

    fn earlier_iteration_events(&self) -> Vec<UsageEvent> {
        let iterations = self.usage.iterations.as_deref().unwrap_or_default();
        let earlier = iterations.split_last().map_or(&[][..], |(_, rest)| rest);
        earlier
            .iter()
            .enumerate()
            .filter_map(|(index, value)| self.iteration_event(index, value))
            .collect()
    }

    fn iteration_event(&self, index: usize, value: &Value) -> Option<UsageEvent> {
        let iteration = RawIteration::deserialize(value).ok()?;
        let tokens = token_counts(&iteration.counts, 0);
        if tokens.total() == Tokens::ZERO {
            return None;
        }
        let model = non_empty(iteration.model).unwrap_or_else(|| self.model.clone());
        let key = match &self.id {
            Some(id) => EventKey(format!("{id}:iter:{index}")),
            None => EventKey::fallback(self.at, &model, &tokens),
        };
        Some(self.event(key, model, tokens, 0))
    }

    fn event(
        &self,
        key: EventKey,
        model: String,
        tokens: TokenCounts,
        web_search_requests: u32,
    ) -> UsageEvent {
        UsageEvent {
            key,
            at: self.at,
            model,
            tier: service_tier(&self.usage),
            tokens,
            web_search_requests,
            reported_cost: None,
        }
    }
}

fn token_counts(counts: &RawCounts, reasoning: u64) -> TokenCounts {
    let (five_minute, one_hour) = match &counts.cache_creation {
        Some(split) => (
            split.ephemeral_5m_input_tokens.unwrap_or(0),
            split.ephemeral_1h_input_tokens.unwrap_or(0),
        ),
        None => (counts.cache_creation_input_tokens.unwrap_or(0), 0),
    };
    TokenCounts {
        input: Tokens(counts.input_tokens),
        cache_read: Tokens(counts.cache_read_input_tokens.unwrap_or(0)),
        cache_write_5m: Tokens(five_minute),
        cache_write_1h: Tokens(one_hour),
        output: Tokens(counts.output_tokens),
        reasoning: Tokens(reasoning),
    }
}

fn thinking_tokens(usage: &RawTokenUsage) -> u64 {
    usage
        .output_tokens_details
        .as_ref()
        .and_then(|details| details.thinking_tokens)
        .unwrap_or(0)
}

fn web_searches(usage: &RawTokenUsage) -> u32 {
    let count = usage
        .server_tool_use
        .as_ref()
        .and_then(|tools| tools.web_search_requests)
        .unwrap_or(0);
    u32::try_from(count).unwrap_or(u32::MAX)
}

fn service_tier(usage: &RawTokenUsage) -> ServiceTier {
    if usage.speed.as_deref() == Some("fast") {
        ServiceTier::Fast
    } else if usage.service_tier.as_deref() == Some("priority") {
        ServiceTier::Priority
    } else {
        ServiceTier::Standard
    }
}

fn non_empty(value: Option<String>) -> Option<String> {
    value.filter(|text| !text.is_empty())
}

#[cfg(test)]
#[path = "log_record_tests.rs"]
mod tests;
