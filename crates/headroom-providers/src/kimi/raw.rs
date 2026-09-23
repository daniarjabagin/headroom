use std::collections::BTreeMap;

use serde::Deserialize;
use serde_json::{Number, Value};

#[derive(Debug, Clone, Default, Deserialize)]
pub(super) struct RawUsages {
    pub(super) user: Option<RawUser>,
    pub(super) usage: Option<Value>,
    pub(super) limits: Option<Vec<Value>>,
    pub(super) usages: Option<BTreeMap<String, Value>>,
    pub(super) data: Option<RawEnvelope>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub(super) struct RawEnvelope {
    pub(super) quota: Option<RawQuota>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub(super) struct RawQuota {
    pub(super) usages: Option<BTreeMap<String, Value>>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub(super) struct RawUser {
    pub(super) membership: Option<RawMembership>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub(super) struct RawMembership {
    pub(super) level: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub(super) struct RawDetail {
    pub(super) limit: RawCount,
    pub(super) used: Option<RawCount>,
    pub(super) remaining: Option<RawCount>,
    #[serde(rename = "resetTime")]
    pub(super) reset_time: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub(super) struct RawLimit {
    pub(super) window: RawWindow,
    pub(super) detail: RawDetail,
}

#[derive(Debug, Clone, Deserialize)]
pub(super) struct RawWindow {
    pub(super) duration: RawCount,
    #[serde(rename = "timeUnit")]
    pub(super) time_unit: String,
}

#[derive(Debug, Clone, Deserialize)]
pub(super) struct RawPool {
    #[serde(alias = "usedRatio")]
    pub(super) used_ratio: RawRatio,
    #[serde(alias = "resetAt")]
    pub(super) reset_time: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
pub(super) enum RawCount {
    Number(Number),
    Text(String),
}

impl RawCount {
    pub(super) fn exact(&self) -> Option<u64> {
        match self {
            RawCount::Number(number) => number.as_u64(),
            RawCount::Text(text) => text.trim().parse().ok(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
pub(super) enum RawRatio {
    Number(f64),
    Text(String),
}

impl RawRatio {
    pub(super) fn value(&self) -> Option<f64> {
        let value = match self {
            RawRatio::Number(value) => *value,
            RawRatio::Text(text) => text.trim().parse().ok()?,
        };
        (value.is_finite() && value >= 0.0).then_some(value)
    }
}
