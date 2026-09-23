use std::collections::BTreeMap;

use serde::Deserialize;
use serde_json::{Number, Value};

#[derive(Debug, Clone, Default, Deserialize)]
pub(super) struct RawUsage {
    pub(super) five_hour: Option<RawWindow>,
    pub(super) seven_day: Option<RawWindow>,
    pub(super) limits: Option<Vec<Value>>,
    pub(super) extra_usage: Option<RawExtraUsage>,
    #[serde(flatten)]
    pub(super) other: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub(super) struct RawWindow {
    pub(super) utilization: Option<f64>,
    pub(super) resets_at: Option<Value>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub(super) struct RawLimit {
    pub(super) kind: Option<String>,
    pub(super) group: Option<String>,
    pub(super) percent: Option<f64>,
    pub(super) resets_at: Option<Value>,
    pub(super) scope: Option<RawScope>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub(super) struct RawScope {
    pub(super) model: Option<RawModelScope>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub(super) struct RawModelScope {
    pub(super) display_name: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub(super) struct RawExtraUsage {
    pub(super) is_enabled: Option<bool>,
    pub(super) used_credits: Option<Number>,
    pub(super) monthly_limit: Option<Number>,
    pub(super) currency: Option<String>,
    pub(super) decimal_places: Option<u32>,
}
