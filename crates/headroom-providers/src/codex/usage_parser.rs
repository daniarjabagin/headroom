use headroom_core::event::{EventKey, ServiceTier, UsageEvent};
use headroom_core::tokens::TokenCounts;
use headroom_core::units::Tokens;
use jiff::{SignedDuration, Timestamp};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::timestamp::parse_log_timestamp;

pub(super) const PAIRING_WINDOW: SignedDuration = SignedDuration::from_mins(15);
const UNKNOWN_MODEL: &str = "unknown";
const INTERESTING: [&str; 4] = [
    "token_count",
    "token_usage_record",
    "turn_context",
    "thread_settings_applied",
];

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
struct RawTokenUsage {
    #[serde(default, rename = "input_tokens", alias = "prompt_tokens")]
    input: u64,
    #[serde(
        default,
        rename = "cached_input_tokens",
        alias = "cache_read_input_tokens",
        alias = "cached_tokens"
    )]
    cached: u64,
    #[serde(default, rename = "cache_write_input_tokens")]
    cache_write: u64,
    #[serde(default, rename = "output_tokens", alias = "completion_tokens")]
    output: u64,
    #[serde(
        default,
        rename = "reasoning_output_tokens",
        alias = "reasoning_tokens"
    )]
    reasoning: u64,
}

impl RawTokenUsage {
    fn saturating_sub(self, earlier: RawTokenUsage) -> RawTokenUsage {
        RawTokenUsage {
            input: self.input.saturating_sub(earlier.input),
            cached: self.cached.saturating_sub(earlier.cached),
            cache_write: self.cache_write.saturating_sub(earlier.cache_write),
            output: self.output.saturating_sub(earlier.output),
            reasoning: self.reasoning.saturating_sub(earlier.reasoning),
        }
    }

    fn token_counts(self) -> TokenCounts {
        TokenCounts {
            input: Tokens(self.input.saturating_sub(self.cached)),
            cache_read: Tokens(self.cached),
            cache_write_5m: Tokens(self.cache_write),
            cache_write_1h: Tokens::ZERO,
            output: Tokens(self.output),
            reasoning: Tokens(self.reasoning),
        }
    }

    fn is_empty(self) -> bool {
        self == RawTokenUsage::default()
    }
}

#[derive(Debug, Deserialize)]
struct RawLine {
    timestamp: Option<String>,
    #[serde(rename = "type")]
    kind: Option<String>,
    payload: Option<RawPayload>,
}

#[derive(Debug, Default, Deserialize)]
struct RawPayload {
    #[serde(rename = "type")]
    kind: Option<String>,
    model: Option<String>,
    service_tier: Option<String>,
    thread_settings: Option<RawThreadSettings>,
    info: Option<RawTokenInfo>,
    response_id: Option<String>,
    usage: Option<RawTokenUsage>,
}

#[derive(Debug, Default, Deserialize)]
struct RawThreadSettings {
    model: Option<String>,
    service_tier: Option<String>,
}

#[derive(Debug, Clone, Copy, Default, Deserialize)]
struct RawTokenInfo {
    total_token_usage: Option<RawTokenUsage>,
    last_token_usage: Option<RawTokenUsage>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub(super) struct ParserState {
    model: Option<String>,
    tier: ServiceTier,
    previous_totals: Option<RawTokenUsage>,
    has_records: bool,
    pending: Vec<UsageEvent>,
    unattributed: Vec<UsageEvent>,
}

impl ParserState {
    pub(super) fn from_value(value: &Value) -> ParserState {
        ParserState::deserialize(value).unwrap_or_default()
    }

    pub(super) fn to_value(&self) -> Result<Value, serde_json::Error> {
        serde_json::to_value(self)
    }

    pub(super) fn consume(&mut self, line: &str, events: &mut Vec<UsageEvent>) {
        if !INTERESTING.iter().any(|needle| line.contains(needle)) {
            return;
        }
        let Ok(raw) = serde_json::from_str::<RawLine>(line) else {
            return;
        };
        let at = raw.timestamp.as_deref().and_then(parse_log_timestamp);
        let payload = raw.payload.unwrap_or_default();
        match (raw.kind.as_deref(), payload.kind.as_deref()) {
            (Some("turn_context"), _) => {
                self.set_model(payload.model);
                self.attribute(events);
            }
            (Some("event_msg"), Some("thread_settings_applied")) => {
                self.apply_settings(payload);
                self.attribute(events);
            }
            (Some("event_msg"), Some("token_count")) => self.on_token_count(at, payload.info),
            (Some("token_usage_record"), _) => self.on_usage_record(at, payload, events),
            _ => {}
        }
    }

    pub(super) fn settle(&mut self, now: Timestamp, events: &mut Vec<UsageEvent>) {
        events.extend(take_settled(&mut self.pending, now));
        events.extend(take_settled(&mut self.unattributed, now));
    }

    fn set_model(&mut self, model: Option<String>) {
        if let Some(model) = model.map(|model| model.trim().to_owned())
            && !model.is_empty()
        {
            self.model = Some(model);
        }
    }

    fn apply_settings(&mut self, payload: RawPayload) {
        let settings = payload.thread_settings.unwrap_or_default();
        self.set_model(settings.model.or(payload.model));
        self.tier = tier(settings.service_tier.or(payload.service_tier).as_deref());
    }

    fn on_usage_record(
        &mut self,
        at: Option<Timestamp>,
        payload: RawPayload,
        events: &mut Vec<UsageEvent>,
    ) {
        self.has_records = true;
        let (Some(at), Some(usage)) = (at, payload.usage) else {
            return;
        };
        let tokens = usage.token_counts();
        self.drop_paired_token_count(&tokens);
        if usage.is_empty() {
            return;
        }
        let response_id = payload.response_id.map(|id| id.trim().to_owned());
        let key = response_id
            .filter(|id| !id.is_empty())
            .map_or_else(|| self.fallback_key(at, &tokens), EventKey);
        let event = self.event(key, at, tokens);
        if self.model.is_some() {
            events.push(event);
        } else {
            self.unattributed.push(event);
        }
    }

    fn attribute(&mut self, events: &mut Vec<UsageEvent>) {
        let Some(model) = self.model.clone() else {
            return;
        };
        let tier = self.tier;
        self.pending
            .iter_mut()
            .filter(|event| event.model == UNKNOWN_MODEL)
            .for_each(|event| attribute_event(event, &model, tier));
        events.extend(self.unattributed.drain(..).map(|mut event| {
            attribute_event(&mut event, &model, tier);
            event
        }));
    }

    fn on_token_count(&mut self, at: Option<Timestamp>, info: Option<RawTokenInfo>) {
        let Some(usage) = info.and_then(|info| self.token_count_usage(info)) else {
            return;
        };
        let Some(at) = at.filter(|_| !self.has_records && !usage.is_empty()) else {
            return;
        };
        let tokens = usage.token_counts();
        let key = self.fallback_key(at, &tokens);
        self.pending.push(self.event(key, at, tokens));
    }

    fn token_count_usage(&mut self, info: RawTokenInfo) -> Option<RawTokenUsage> {
        let totals = info.total_token_usage;
        if totals.is_some() && totals == self.previous_totals {
            return None;
        }
        let delta =
            totals.map(|totals| totals.saturating_sub(self.previous_totals.unwrap_or_default()));
        if totals.is_some() {
            self.previous_totals = totals;
        }
        info.last_token_usage.or(delta)
    }

    fn drop_paired_token_count(&mut self, tokens: &TokenCounts) {
        if let Some(index) = self
            .pending
            .iter()
            .position(|event| event.tokens == *tokens)
        {
            self.pending.remove(index);
        }
    }

    fn model(&self) -> String {
        self.model
            .clone()
            .unwrap_or_else(|| UNKNOWN_MODEL.to_owned())
    }

    fn fallback_key(&self, at: Timestamp, tokens: &TokenCounts) -> EventKey {
        EventKey::fallback(at, &self.model(), tokens)
    }

    fn event(&self, key: EventKey, at: Timestamp, tokens: TokenCounts) -> UsageEvent {
        UsageEvent {
            key,
            at,
            model: self.model(),
            tier: self.tier,
            tokens,
            web_search_requests: 0,
        }
    }
}

fn attribute_event(event: &mut UsageEvent, model: &str, tier: ServiceTier) {
    if event.key.is_fallback() {
        event.key = EventKey::fallback(event.at, model, &event.tokens);
    }
    model.clone_into(&mut event.model);
    event.tier = tier;
}

fn take_settled(events: &mut Vec<UsageEvent>, now: Timestamp) -> Vec<UsageEvent> {
    let (settled, waiting) = std::mem::take(events)
        .into_iter()
        .partition(|event| is_settled(event.at, now));
    *events = waiting;
    settled
}

fn is_settled(at: Timestamp, now: Timestamp) -> bool {
    at.checked_add(PAIRING_WINDOW)
        .is_ok_and(|deadline| deadline <= now)
}

fn tier(raw: Option<&str>) -> ServiceTier {
    match raw.map(str::trim) {
        Some(tier) if tier.eq_ignore_ascii_case("priority") => ServiceTier::Priority,
        Some(tier) if tier.eq_ignore_ascii_case("fast") => ServiceTier::Fast,
        _ => ServiceTier::Standard,
    }
}

#[cfg(test)]
#[path = "usage_parser_tests.rs"]
mod tests;
