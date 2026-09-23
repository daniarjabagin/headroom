use serde::Deserialize;
use serde_json::{Number, Value};

#[derive(Debug, Clone, Default, Deserialize)]
pub(super) struct RawUser {
    pub(super) copilot_plan: Option<String>,
    pub(super) quota_reset_date: Option<String>,
    pub(super) limited_user_reset_date: Option<String>,
    pub(super) quota_snapshots: Option<RawSnapshots>,
    pub(super) limited_user_quotas: Option<RawCounts>,
    pub(super) monthly_quotas: Option<RawCounts>,
    pub(super) token_based_billing: Option<Value>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub(super) struct RawSnapshots {
    pub(super) premium_interactions: Option<RawSnapshot>,
    pub(super) premium_requests: Option<RawSnapshot>,
    pub(super) chat: Option<RawSnapshot>,
    pub(super) completions: Option<RawSnapshot>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub(super) struct RawSnapshot {
    pub(super) entitlement: Option<Number>,
    pub(super) remaining: Option<Number>,
    pub(super) percent_remaining: Option<Number>,
    pub(super) unlimited: Option<bool>,
    pub(super) overage_permitted: Option<bool>,
    pub(super) overage_count: Option<Number>,
    pub(super) credits_used: Option<Number>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub(super) struct RawCounts {
    pub(super) chat: Option<Number>,
    pub(super) completions: Option<Number>,
}
