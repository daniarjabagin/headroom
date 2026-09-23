use headroom_core::event::{EventKey, ServiceTier, UsageEvent};
use headroom_core::tokens::TokenCounts;
use headroom_core::units::Tokens;
use jiff::Timestamp;
use serde::Deserialize;

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
    input_tokens: u64,
    output_tokens: u64,
    cache_read_input_tokens: Option<u64>,
    cache_creation_input_tokens: Option<u64>,
    cache_creation: Option<RawCacheCreation>,
    output_tokens_details: Option<RawOutputDetails>,
    server_tool_use: Option<RawServerToolUse>,
    service_tier: Option<String>,
    speed: Option<String>,
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

pub(super) fn parse_line(line: &str) -> Option<UsageEvent> {
    let raw: RawLine = serde_json::from_str(line).ok()?;
    if raw.kind.as_deref() != Some(ASSISTANT) {
        return None;
    }
    let message = raw.message?;
    let usage = message.usage?;
    let model = non_empty(message.model).filter(|model| model != SYNTHETIC_MODEL)?;
    let at: Timestamp = raw.timestamp?.parse().ok()?;
    let tokens = token_counts(&usage);
    let web_search_requests = web_searches(&usage);
    if tokens.total() == Tokens::ZERO && web_search_requests == 0 {
        return None;
    }
    let key = match (non_empty(message.id), non_empty(raw.request_id)) {
        (Some(id), Some(request)) => EventKey(format!("{id}:{request}")),
        _ => EventKey::fallback(at, &model, &tokens),
    };
    Some(UsageEvent {
        key,
        at,
        tier: service_tier(&usage),
        model,
        tokens,
        web_search_requests,
    })
}

fn token_counts(usage: &RawTokenUsage) -> TokenCounts {
    let (five_minute, one_hour) = match &usage.cache_creation {
        Some(split) => (
            split.ephemeral_5m_input_tokens.unwrap_or(0),
            split.ephemeral_1h_input_tokens.unwrap_or(0),
        ),
        None => (usage.cache_creation_input_tokens.unwrap_or(0), 0),
    };
    TokenCounts {
        input: Tokens(usage.input_tokens),
        cache_read: Tokens(usage.cache_read_input_tokens.unwrap_or(0)),
        cache_write_5m: Tokens(five_minute),
        cache_write_1h: Tokens(one_hour),
        output: Tokens(usage.output_tokens),
        reasoning: Tokens(
            usage
                .output_tokens_details
                .as_ref()
                .and_then(|details| details.thinking_tokens)
                .unwrap_or(0),
        ),
    }
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
