use jiff::Timestamp;
use serde::Deserialize;

use super::Tone;
use super::recovery::RecoveryField;

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Account {
    pub id: String,
    pub provider: String,
    pub provider_name: String,
    pub label: Option<String>,
    pub email: Option<String>,
    pub plan: Option<String>,
    pub hidden: bool,
    #[serde(default)]
    pub owner: Owner,
    pub status: Status,
    pub error: Option<AccountError>,
    #[serde(default)]
    pub recovery: RecoveryField,
    pub updated_at: Option<Timestamp>,
    pub windows: Vec<Window>,
    pub balances: Vec<Balance>,
    pub notices: Vec<Notice>,
    pub usage_home: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Owner {
    Headroom,
    #[default]
    #[serde(other)]
    Cli,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Fresh,
    Refreshing,
    Error,
    SignedOut,
    NoSubscription,
    #[serde(other)]
    Stale,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct AccountError {
    pub kind: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Window {
    pub id: String,
    pub label: String,
    pub used_percent: f64,
    pub remaining_percent: f64,
    pub resets_at: Option<Timestamp>,
    pub tone: Tone,
    pub pace: Pace,
    pub hidden: bool,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Pace {
    pub severity: Severity,
    pub even_pace_percent: Option<f64>,
    pub projected_percent: Option<f64>,
    pub spare_percent: Option<f64>,
    pub runs_out_at: Option<Timestamp>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Healthy,
    Close,
    RunningOut,
    Spent,
    #[serde(other)]
    Untracked,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Balance {
    pub id: String,
    pub label: String,
    #[serde(flatten)]
    pub amount: BalanceAmount,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum BalanceAmount {
    Usd {
        usd_micros: i64,
    },
    Money {
        currency: String,
        micros: i64,
    },
    Count {
        value: u64,
        unit: String,
    },
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Notice {
    pub tone: Tone,
    pub text: String,
}
