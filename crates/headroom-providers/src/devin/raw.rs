use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct RawStatusResponse {
    pub(super) user_status: RawUserStatus,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct RawUserStatus {
    pub(super) email: Option<String>,
    pub(super) plan_status: Option<RawPlanStatus>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct RawPlanStatus {
    pub(super) plan_info: Option<RawPlanInfo>,
    pub(super) daily_quota_remaining_percent: Option<Value>,
    pub(super) weekly_quota_remaining_percent: Option<Value>,
    pub(super) daily_quota_reset_at_unix: Option<Value>,
    pub(super) weekly_quota_reset_at_unix: Option<Value>,
    pub(super) overage_balance_micros: Option<Value>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct RawPlanInfo {
    pub(super) plan_name: Option<String>,
    pub(super) hide_daily_quota: Option<Value>,
}
