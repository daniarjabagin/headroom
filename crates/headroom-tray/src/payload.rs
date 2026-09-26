use jiff::Timestamp;
use serde::Deserialize;

use crate::combined::CombinedGroup;

mod account;
mod diagnostics;
mod display;
mod lenient;
mod panel;
mod projects;
mod recovery;
mod spend_query;
mod status;
mod usage;

pub use account::{
    Account, AccountError, Balance, BalanceAmount, Notice, Owner, Pace, PaceBasis, Refresh,
    RefreshMode, RefreshReason, Severity, Status, Window,
};
pub use diagnostics::{Diagnostics, parse_diagnostics};
pub use display::{
    Density, Display, Language, PanelBox, PanelIndicator, PanelLabel, PanelLimit, PanelMode,
    PanelPosition, ResetFormat, SpendBreakdown, SpendPeriod, SpendUnit, Theme, TimeFormat,
    ValueMode,
};
pub use panel::{Headline, PanelItem};
pub use projects::{OtherProjects, ProjectProvider, ProjectSpend};
pub use recovery::{Recovery, RecoveryField};
pub use spend_query::{
    SpendGroup, SpendQuery, SpendRange, SpendReport, SpendRow, parse_spend_report,
};
pub use status::{ProviderStatus, StatusIndicator};
pub use usage::{
    Daily, ModelUsage, OtherModels, PeriodSpend, ProviderSpend, Spend, Tokens, Totals, Usage,
};

use lenient::{lenient, lenient_items, lenient_list};

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
    #[serde(default, deserialize_with = "lenient")]
    pub update_check: Option<UpdateCheck>,
    pub display: Display,
    pub headline: Option<Headline>,
    #[serde(default, deserialize_with = "lenient_items")]
    pub panel_items: Option<Vec<PanelItem>>,
    #[serde(default, deserialize_with = "lenient")]
    pub panel_tone: Option<Tone>,
    pub accounts: Vec<Account>,
    pub usage: Vec<Usage>,
    pub spend: Spend,
    #[serde(default)]
    pub combined: Vec<CombinedGroup>,
    #[serde(default, deserialize_with = "lenient_list")]
    pub provider_status: Vec<ProviderStatus>,
}

impl State {
    #[must_use]
    pub fn is_refreshing(&self) -> bool {
        self.accounts
            .iter()
            .any(|account| account.status == Status::Refreshing)
    }

    #[must_use]
    pub fn speaks_0_6(&self) -> bool {
        self.panel_items.is_some()
    }

    #[must_use]
    pub fn resolved_panel_items(&self) -> Vec<PanelItem> {
        match &self.panel_items {
            Some(items) => items.clone(),
            None => self
                .headline
                .iter()
                .map(|headline| PanelItem::from_headline(headline, self.display.value_mode))
                .collect(),
        }
    }

    #[must_use]
    pub fn provider_status_of(&self, provider: &str) -> Option<&ProviderStatus> {
        self.provider_status
            .iter()
            .find(|status| status.provider == provider)
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

#[cfg(test)]
#[path = "payload_release_tests.rs"]
mod release_tests;
