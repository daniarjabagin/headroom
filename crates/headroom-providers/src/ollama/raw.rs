use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub(super) struct RawUsage {
    pub limits: RawLimits,
    pub activity: Option<RawActivity>,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
pub(super) struct RawLimits {
    pub session: Option<RawLimit>,
    pub weekly: Option<RawLimit>,
    pub monthly: Option<RawLimit>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub(super) struct RawLimit {
    pub usage: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub(super) struct RawActivity {
    pub cost: Option<Value>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
pub(super) struct RawMe {
    #[serde(alias = "Email")]
    pub email: Option<String>,
    #[serde(alias = "Plan")]
    pub plan: Option<String>,
}
