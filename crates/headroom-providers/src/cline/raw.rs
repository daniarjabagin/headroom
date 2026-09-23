use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(super) struct RawUser {
    pub(super) id: String,
    pub(super) email: Option<String>,
    #[serde(default)]
    pub(super) organizations: Option<Vec<RawOrganization>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct RawOrganization {
    pub(super) organization_id: String,
    pub(super) name: Option<String>,
    #[serde(default)]
    pub(super) active: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct RawCurrentPlan {
    pub(super) plan: Option<RawPlan>,
    pub(super) current_period_end: Option<String>,
    pub(super) cancel_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct RawPlan {
    pub(super) id: Option<String>,
    pub(super) name: Option<String>,
    pub(super) display_name: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub(super) struct RawBalance {
    pub(super) balance: i64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct RawTokens {
    pub(super) access_token: String,
    pub(super) refresh_token: Option<String>,
    pub(super) expires_at: String,
}
