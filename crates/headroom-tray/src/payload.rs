use std::collections::BTreeMap;

use jiff::Timestamp;
use serde::Deserialize;

use crate::combined::CombinedGroup;

mod account;
mod recovery;
mod usage;

pub use account::{
    Account, AccountError, Balance, BalanceAmount, Notice, Owner, Pace, Severity, Status, Window,
};
pub use recovery::{Recovery, RecoveryField};
pub use usage::{
    Daily, ModelUsage, OtherModels, PeriodSpend, ProviderSpend, Spend, Tokens, Totals, Usage,
};

pub const STATE_VERSION: u32 = 1;

#[derive(Debug, thiserror::Error)]
pub enum PayloadError {
    #[error("unreadable state from the Headroom service: {0}")]
    Json(#[from] serde_json::Error),
    #[error("the Headroom service speaks state version {0}, expected {STATE_VERSION}")]
    Version(u32),
}

#[derive(Debug, Deserialize)]
struct Envelope {
    version: u32,
}

pub fn parse_state(json: &str) -> Result<State, PayloadError> {
    let envelope: Envelope = serde_json::from_str(json)?;
    if envelope.version != STATE_VERSION {
        return Err(PayloadError::Version(envelope.version));
    }
    Ok(serde_json::from_str(json)?)
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct State {
    #[serde(default)]
    pub app_version: Option<String>,
    pub generated_at: Timestamp,
    pub next_refresh_at: Option<Timestamp>,
    pub last_success_at: Option<Timestamp>,
    pub offline: bool,
    #[serde(default)]
    pub update: Option<Update>,
    #[serde(default, deserialize_with = "recovery::lenient")]
    pub update_check: Option<UpdateCheck>,
    pub display: Display,
    pub headline: Option<Headline>,
    pub accounts: Vec<Account>,
    pub usage: Vec<Usage>,
    pub spend: Spend,
    #[serde(default)]
    pub combined: Vec<CombinedGroup>,
}

impl State {
    #[must_use]
    pub fn is_refreshing(&self) -> bool {
        self.accounts
            .iter()
            .any(|account| account.status == Status::Refreshing)
    }

    #[must_use]
    pub fn usage_of(&self, account: &Account) -> Option<&Usage> {
        self.usage.iter().find(|usage| {
            usage.provider == account.provider && usage.usage_home == account.usage_home
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Update {
    pub version: String,
    pub url: String,
    pub install: InstallKind,
    pub command: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct UpdateCheck {
    #[serde(default)]
    pub checked_at: Option<Timestamp>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InstallKind {
    #[serde(rename = "self")]
    SelfManaged,
    Package,
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "each flag is an independent user toggle from the daemon"
)]
pub struct Display {
    pub theme: Theme,
    pub language: Language,
    pub value_mode: ValueMode,
    pub reset_format: ResetFormat,
    pub show_spend: bool,
    pub show_account_spend: bool,
    pub show_trend: bool,
    pub show_forecast: bool,
    pub hidden_windows: BTreeMap<String, Vec<String>>,
}

impl Default for Display {
    fn default() -> Self {
        Self {
            theme: Theme::System,
            language: Language::System,
            value_mode: ValueMode::Left,
            reset_format: ResetFormat::Countdown,
            show_spend: true,
            show_account_spend: true,
            show_trend: true,
            show_forecast: true,
            hidden_windows: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Theme {
    Light,
    Dark,
    #[serde(other)]
    System,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Language {
    En,
    Ru,
    #[serde(other)]
    System,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValueMode {
    Used,
    #[serde(other)]
    Left,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResetFormat {
    Exact,
    #[serde(other)]
    Countdown,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Headline {
    #[serde(default)]
    pub account_id: Option<String>,
    pub provider: String,
    pub provider_name: String,
    #[serde(default)]
    pub account_label: Option<String>,
    #[serde(default)]
    pub combined: bool,
    #[serde(default)]
    pub account_count: Option<u32>,
    pub window: String,
    pub window_label: String,
    pub used_percent: f64,
    pub remaining_percent: f64,
    pub tone: Tone,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Tone {
    Good,
    Warning,
    Critical,
    #[serde(other)]
    Neutral,
}

#[cfg(test)]
#[path = "payload_tests.rs"]
mod tests;
