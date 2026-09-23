use jiff::Timestamp;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::tokens::TokenCounts;
use crate::units::MicroUsd;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UsageEvent {
    pub key: EventKey,
    pub at: Timestamp,
    pub model: String,
    pub tier: ServiceTier,
    pub tokens: TokenCounts,
    pub web_search_requests: u32,
    /// Exact cost the provider logged for this event; it takes precedence over the price book.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reported_cost: Option<MicroUsd>,
}

#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum ServiceTier {
    #[default]
    Standard,
    Priority,
    Fast,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EventKey(pub String);

impl EventKey {
    const FALLBACK_PREFIX: &'static str = "fallback:";

    #[must_use]
    pub fn fallback(at: Timestamp, model: &str, tokens: &TokenCounts) -> EventKey {
        let digest = Sha256::digest(fallback_material(at, model, tokens).as_bytes());
        EventKey(format!("{}{}", Self::FALLBACK_PREFIX, hex::encode(digest)))
    }

    #[must_use]
    pub fn is_fallback(&self) -> bool {
        self.0.starts_with(Self::FALLBACK_PREFIX)
    }
}

fn fallback_material(at: Timestamp, model: &str, tokens: &TokenCounts) -> String {
    format!(
        "{}|{}|{}|{}|{}|{}|{}|{}|{}",
        at.as_second(),
        at.subsec_nanosecond(),
        model,
        tokens.input.0,
        tokens.cache_read.0,
        tokens.cache_write_5m.0,
        tokens.cache_write_1h.0,
        tokens.output.0,
        tokens.reasoning.0,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::units::Tokens;

    fn tokens(output: u64) -> TokenCounts {
        TokenCounts {
            input: Tokens(10),
            output: Tokens(output),
            ..TokenCounts::default()
        }
    }

    fn at(text: &str) -> Timestamp {
        text.parse().unwrap()
    }

    #[test]
    fn fallback_is_deterministic() {
        let first = EventKey::fallback(at("2026-09-23T10:00:00Z"), "gpt-5.5", &tokens(5));
        let second = EventKey::fallback(at("2026-09-23T10:00:00Z"), "gpt-5.5", &tokens(5));
        assert_eq!(first, second);
        assert!(first.is_fallback());
        assert_eq!(first.0.len(), "fallback:".len() + 64);
    }

    #[test]
    fn fallback_differs_on_every_input() {
        let base = EventKey::fallback(at("2026-09-23T10:00:00Z"), "gpt-5.5", &tokens(5));
        let later = EventKey::fallback(at("2026-09-23T10:00:00.001Z"), "gpt-5.5", &tokens(5));
        let other_model = EventKey::fallback(at("2026-09-23T10:00:00Z"), "gpt-5", &tokens(5));
        let other_tokens = EventKey::fallback(at("2026-09-23T10:00:00Z"), "gpt-5.5", &tokens(6));
        assert_ne!(base, later);
        assert_ne!(base, other_model);
        assert_ne!(base, other_tokens);
    }

    #[test]
    fn fallback_ignores_timestamp_offset_spelling() {
        let utc = EventKey::fallback(at("2026-09-23T10:00:00Z"), "m", &tokens(1));
        let offset = EventKey::fallback(at("2026-09-23T15:00:00+05:00"), "m", &tokens(1));
        assert_eq!(utc, offset);
    }

    #[test]
    fn provider_keys_are_not_fallback() {
        assert!(!EventKey("resp_123".into()).is_fallback());
    }

    #[test]
    fn service_tier_serializes_snake_case() {
        assert_eq!(
            serde_json::to_string(&ServiceTier::Priority).unwrap(),
            "\"priority\""
        );
    }
}
