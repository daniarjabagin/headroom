use serde::Deserialize;

use super::money::Usd;

#[derive(Debug, Deserialize)]
pub(super) struct Envelope<T> {
    pub(super) data: T,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(super) struct RawKey {
    pub(super) usage: Option<Usd>,
    pub(super) usage_daily: Option<Usd>,
    pub(super) usage_weekly: Option<Usd>,
    pub(super) usage_monthly: Option<Usd>,
    pub(super) limit: Option<Usd>,
    pub(super) limit_remaining: Option<Usd>,
    pub(super) limit_reset: Option<String>,
    pub(super) is_free_tier: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(super) struct RawCredits {
    pub(super) total_credits: Usd,
    pub(super) total_usage: Usd,
}
