use serde::Deserialize;

use crate::decimal::ExactMicros;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(super) struct RawBalance {
    pub(super) is_available: bool,
    pub(super) balance_infos: Vec<RawBalanceInfo>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(super) struct RawBalanceInfo {
    pub(super) currency: String,
    pub(super) total_balance: ExactMicros,
}
