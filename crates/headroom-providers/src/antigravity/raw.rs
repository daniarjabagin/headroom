use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub(super) struct RawSummaryEnvelope {
    pub response: Option<RawSummary>,
    pub groups: Option<Vec<RawGroup>>,
}

impl RawSummaryEnvelope {
    pub(super) fn groups(&self) -> Option<&[RawGroup]> {
        self.response
            .as_ref()
            .and_then(|response| response.groups.as_deref())
            .or(self.groups.as_deref())
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub(super) struct RawSummary {
    pub groups: Option<Vec<RawGroup>>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub(super) struct RawGroup {
    #[serde(default)]
    pub buckets: Vec<Value>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct RawBucket {
    pub bucket_id: String,
    pub remaining_fraction: f64,
    pub reset_time: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct RawUserStatusEnvelope {
    pub user_status: Option<RawUserStatus>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct RawUserStatus {
    pub user_tier: Option<RawTier>,
    pub plan_status: Option<RawPlanStatus>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct RawPlanStatus {
    pub plan_info: Option<RawPlanInfo>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct RawPlanInfo {
    pub plan_name: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
pub(super) struct RawTier {
    pub name: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct RawCodeAssist {
    pub paid_tier: Option<RawTier>,
    pub current_tier: Option<RawTier>,
}
