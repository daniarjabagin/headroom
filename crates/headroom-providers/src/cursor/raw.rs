use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct RawPeriodUsage {
    pub(super) enabled: Option<bool>,
    pub(super) billing_cycle_start: Option<Value>,
    pub(super) billing_cycle_end: Option<Value>,
    pub(super) plan_usage: Option<RawPlanUsage>,
    pub(super) spend_limit_usage: Option<RawSpendLimitUsage>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct RawPlanUsage {
    pub(super) limit: Option<Value>,
    pub(super) total_spend: Option<Value>,
    pub(super) remaining: Option<Value>,
    pub(super) total_percent_used: Option<Value>,
    pub(super) auto_percent_used: Option<Value>,
    pub(super) api_percent_used: Option<Value>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct RawSpendLimitUsage {
    pub(super) limit_type: Option<String>,
    pub(super) individual_limit: Option<Value>,
    pub(super) pooled_limit: Option<Value>,
    pub(super) individual_remaining: Option<Value>,
    pub(super) pooled_remaining: Option<Value>,
    pub(super) individual_used: Option<Value>,
    pub(super) pooled_used: Option<Value>,
    pub(super) total_spend: Option<Value>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct RawPlanInfoResponse {
    pub(super) plan_info: Option<RawPlanInfo>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct RawPlanInfo {
    pub(super) plan_name: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct RawCreditGrants {
    pub(super) has_credit_grants: Option<bool>,
    pub(super) total_cents: Option<Value>,
    pub(super) used_cents: Option<Value>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct RawStripe {
    pub(super) customer_balance: Option<Value>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct RawGrokBotUsage {
    pub(super) uses_pooled_enterprise_allowance: Option<bool>,
    pub(super) has_non_zero_included_limit: Option<bool>,
    pub(super) included_limit_zero: Option<bool>,
    pub(super) usage_percent: Option<Value>,
    pub(super) current_period_start: Option<String>,
    pub(super) next_reset_timestamp_utc: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub(super) struct RawRequestUsage {
    #[serde(rename = "gpt-4")]
    pub(super) gpt4: Option<RawRequestModel>,
    #[serde(rename = "startOfMonth")]
    pub(super) start_of_month: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct RawRequestModel {
    pub(super) num_requests: Option<Value>,
    pub(super) num_requests_total: Option<Value>,
    pub(super) max_request_usage: Option<Value>,
}
