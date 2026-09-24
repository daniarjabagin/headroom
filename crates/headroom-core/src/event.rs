use jiff::Timestamp;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::tokens::TokenCounts;
use crate::units::{MicroUsd, Tokens};

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

impl UsageEvent {
    /// The timestamp fits in signed 64-bit nanoseconds and every token count, the total included, in `i64`.
    #[must_use]
    pub fn fits_in_i64(&self) -> bool {
        let tokens = &self.tokens;
        let counts = [
            tokens.input,
            tokens.cache_read,
            tokens.cache_write_5m,
            tokens.cache_write_1h,
            tokens.output,
            tokens.reasoning,
            tokens.total(),
        ];
        i64::try_from(self.at.as_nanosecond()).is_ok() && counts.into_iter().all(fits_in_i64)
    }
}

fn fits_in_i64(count: Tokens) -> bool {
    i64::try_from(count.0).is_ok()
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

    fn usage(at_text: &str, tokens: TokenCounts) -> UsageEvent {
        UsageEvent {
            key: EventKey("e".into()),
            at: at(at_text),
            model: "m".into(),
            tier: ServiceTier::Standard,
            tokens,
            web_search_requests: 0,
            reported_cost: None,
        }
    }

    #[test]
    fn events_fit_in_i64_until_the_nanosecond_range_or_token_counts_overflow() {
        let max = Tokens(u64::try_from(i64::MAX).unwrap());
        assert!(usage("2026-09-23T10:00:00Z", tokens(5)).fits_in_i64());
        assert!(usage("2262-04-11T23:47:16Z", tokens(5)).fits_in_i64());
        assert!(usage("1677-09-21T00:12:44Z", tokens(5)).fits_in_i64());
        assert!(!usage("2262-04-11T23:47:17Z", tokens(5)).fits_in_i64());
        assert!(!usage("1677-09-21T00:12:43Z", tokens(5)).fits_in_i64());
        let biggest = TokenCounts {
            input: max,
            ..TokenCounts::default()
        };
        assert!(usage("2026-09-23T10:00:00Z", biggest).fits_in_i64());
        let too_big = TokenCounts {
            reasoning: Tokens(max.0 + 1),
            ..TokenCounts::default()
        };
        assert!(!usage("2026-09-23T10:00:00Z", too_big).fits_in_i64());
        let overflowing_total = TokenCounts {
            input: max,
            output: Tokens(1),
            ..TokenCounts::default()
        };
        assert!(!usage("2026-09-23T10:00:00Z", overflowing_total).fits_in_i64());
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
