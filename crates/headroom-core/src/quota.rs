use jiff::{SignedDuration, Timestamp};
use serde::{Deserialize, Serialize};

use crate::account::AccountIdentity;
use crate::pace::Tone;
use crate::units::{MicroUsd, Percent};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QuotaWindow {
    pub id: WindowId,
    pub label: String,
    pub used: Percent,
    pub resets_at: Option<Timestamp>,
    pub period: Option<SignedDuration>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WindowId {
    Session,
    Weekly,
    Model(String),
    Other(String),
}

impl WindowId {
    pub const SESSION_PERIOD: SignedDuration = SignedDuration::from_hours(5);
    pub const WEEKLY_PERIOD: SignedDuration = SignedDuration::from_hours(7 * 24);

    #[must_use]
    pub fn from_period(period: SignedDuration) -> Option<WindowId> {
        if period == Self::SESSION_PERIOD {
            Some(WindowId::Session)
        } else if period == Self::WEEKLY_PERIOD {
            Some(WindowId::Weekly)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Balance {
    pub id: String,
    pub label: String,
    pub amount: BalanceAmount,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BalanceAmount {
    Usd(MicroUsd),
    Count { value: u64, unit: String },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LimitsSnapshot {
    pub identity: AccountIdentity,
    pub windows: Vec<QuotaWindow>,
    pub balances: Vec<Balance>,
    pub notices: Vec<Notice>,
    pub fetched_at: Timestamp,
    pub source: LimitsSource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum LimitsSource {
    Live,
    LocalLog { observed_at: Timestamp },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Notice {
    pub tone: Tone,
    pub text: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn five_hours_is_session() {
        assert_eq!(
            WindowId::from_period(SignedDuration::from_secs(18_000)),
            Some(WindowId::Session)
        );
    }

    #[test]
    fn seven_days_is_weekly() {
        assert_eq!(
            WindowId::from_period(SignedDuration::from_secs(604_800)),
            Some(WindowId::Weekly)
        );
        assert_eq!(
            WindowId::from_period(SignedDuration::from_mins(10_080)),
            Some(WindowId::Weekly)
        );
    }

    #[test]
    fn other_lengths_are_unclassified() {
        for secs in [0, 3_600, 17_999, 18_001, 86_400, 2_592_000] {
            assert_eq!(WindowId::from_period(SignedDuration::from_secs(secs)), None);
        }
    }

    #[test]
    fn window_id_serializes_snake_case() {
        assert_eq!(
            serde_json::to_string(&WindowId::Session).unwrap(),
            "\"session\""
        );
        assert_eq!(
            serde_json::to_string(&WindowId::Model("sonnet".into())).unwrap(),
            "{\"model\":\"sonnet\"}"
        );
    }

    #[test]
    fn limits_source_serializes_tagged() {
        let source = LimitsSource::LocalLog {
            observed_at: "2026-09-23T10:00:00Z".parse().unwrap(),
        };
        let json = serde_json::to_string(&source).unwrap();
        assert_eq!(
            json,
            "{\"kind\":\"local_log\",\"observed_at\":\"2026-09-23T10:00:00Z\"}"
        );
        assert_eq!(
            serde_json::to_string(&LimitsSource::Live).unwrap(),
            "{\"kind\":\"live\"}"
        );
    }
}
