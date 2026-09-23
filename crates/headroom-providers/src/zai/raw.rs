use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Deserialize)]
pub(super) struct QuotaReply {
    pub(super) success: Option<bool>,
    pub(super) code: Option<Value>,
    pub(super) msg: Option<String>,
    pub(super) data: Option<QuotaData>,
    pub(super) limits: Option<Vec<RawLimit>>,
}

#[derive(Debug, Deserialize)]
pub(super) struct QuotaData {
    pub(super) limits: Option<Vec<RawLimit>>,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct RawLimit {
    #[serde(rename = "type")]
    pub(super) kind: Option<String>,
    pub(super) name: Option<String>,
    pub(super) unit: Option<i64>,
    pub(super) number: Option<i64>,
    pub(super) percentage: Option<f64>,
    pub(super) next_reset_time: Option<i64>,
    pub(super) usage: Option<u64>,
    pub(super) current_value: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub(super) struct SubscriptionReply {
    pub(super) data: Option<Vec<RawSubscription>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct RawSubscription {
    pub(super) product_name: Option<String>,
}
