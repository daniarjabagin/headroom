use serde::Deserialize;

use crate::decimal::ExactMicros;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(super) struct RawEnvelope {
    pub(super) code: i64,
    pub(super) status: bool,
    pub(super) data: Option<RawBalance>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(super) struct RawBalance {
    #[serde(rename = "available_balance")]
    pub(super) available: ExactMicros,
    #[serde(rename = "voucher_balance")]
    pub(super) voucher: ExactMicros,
    #[serde(rename = "cash_balance")]
    pub(super) cash: ExactMicros,
}
