use serde::Deserialize;
use serde_json::{Number, Value};

#[derive(Debug, Clone, Default, Deserialize)]
pub(super) struct RawRemains {
    pub(super) base_resp: Option<RawBaseResp>,
    pub(super) model_remains: Option<Vec<Value>>,
}

#[derive(Debug, Clone, Deserialize)]
pub(super) struct RawBaseResp {
    pub(super) status_code: i64,
    pub(super) status_msg: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub(super) struct RawModel {
    pub(super) model_name: Option<String>,
    pub(super) start_time: Option<RawNumber>,
    pub(super) end_time: Option<RawNumber>,
    pub(super) current_interval_status: Option<RawNumber>,
    pub(super) current_interval_remaining_percent: Option<RawNumber>,
    pub(super) weekly_start_time: Option<RawNumber>,
    pub(super) weekly_end_time: Option<RawNumber>,
    pub(super) current_weekly_status: Option<RawNumber>,
    pub(super) current_weekly_remaining_percent: Option<RawNumber>,
    pub(super) weekly_boost_permille: Option<RawNumber>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
pub(super) enum RawNumber {
    Number(Number),
    Text(String),
}

impl RawNumber {
    pub(super) fn integer(&self) -> Option<i64> {
        match self {
            RawNumber::Number(number) => number.as_i64(),
            RawNumber::Text(text) => text.trim().parse().ok(),
        }
    }

    pub(super) fn decimal(&self) -> Option<f64> {
        let value = match self {
            RawNumber::Number(number) => number.as_f64()?,
            RawNumber::Text(text) => text.trim().parse().ok()?,
        };
        value.is_finite().then_some(value)
    }
}
