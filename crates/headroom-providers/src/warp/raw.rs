use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub(super) struct RawResponse {
    pub(super) data: Option<RawData>,
    #[serde(default)]
    pub(super) errors: Vec<RawMessage>,
}

#[derive(Debug, Deserialize)]
pub(super) struct RawData {
    pub(super) user: RawUserResult,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "__typename")]
pub(super) enum RawUserResult {
    UserOutput {
        user: RawUser,
    },
    UserFacingError {
        error: RawMessage,
    },
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(super) struct RawMessage {
    pub(super) message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct RawUser {
    pub(super) request_limit_info: RawRequestLimitInfo,
    pub(super) bonus_grants: Option<Vec<RawGrant>>,
    pub(super) workspaces: Option<Vec<RawWorkspace>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct RawRequestLimitInfo {
    pub(super) is_unlimited: bool,
    pub(super) next_refresh_time: Option<String>,
    pub(super) request_limit: u64,
    pub(super) requests_used_since_last_refresh: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct RawGrant {
    pub(super) request_credits_remaining: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct RawWorkspace {
    pub(super) bonus_grants_info: Option<RawGrantsInfo>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(super) struct RawGrantsInfo {
    pub(super) grants: Option<Vec<RawGrant>>,
}
