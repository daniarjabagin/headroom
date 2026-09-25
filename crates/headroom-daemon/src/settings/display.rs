use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::panel::{self, PanelIndicator, PanelLabel, PanelLimit, PanelMode, PanelPosition};
use crate::error::SettingsError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "each flag is an independent user toggle"
)]
pub struct DisplaySettings {
    pub theme: Theme,
    pub language: Language,
    pub value_mode: ValueMode,
    pub reset_format: ResetFormat,
    pub panel_label: PanelLabel,
    pub show_spend: bool,
    pub show_account_spend: bool,
    pub show_trend: bool,
    pub show_forecast: bool,
    pub translucent: bool,
    pub combine_accounts: bool,
    pub hidden_windows: BTreeMap<String, Vec<String>>,
    pub density: Density,
    pub time_format: TimeFormat,
    pub panel_mode: PanelMode,
    pub panel_indicator: PanelIndicator,
    pub panel_limits: Vec<PanelLimit>,
    pub panel_position: PanelPosition,
    pub spend_period: SpendPeriod,
    pub spend_unit: SpendUnit,
    pub spend_breakdown: SpendBreakdown,
    pub starred_accounts: Vec<String>,
    pub collapse_unstarred: bool,
    pub hide_on_screen_share: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Theme {
    #[default]
    System,
    Light,
    Dark,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Language {
    #[default]
    System,
    En,
    Ru,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValueMode {
    #[default]
    Left,
    Used,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResetFormat {
    #[default]
    Countdown,
    Exact,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Density {
    #[default]
    Normal,
    Compact,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum TimeFormat {
    #[default]
    #[serde(rename = "auto")]
    Auto,
    #[serde(rename = "12h")]
    Hour12,
    #[serde(rename = "24h")]
    Hour24,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpendPeriod {
    #[serde(rename = "today")]
    Today,
    #[serde(rename = "yesterday")]
    Yesterday,
    #[serde(rename = "7d")]
    Last7Days,
    #[default]
    #[serde(rename = "30d")]
    Last30Days,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpendUnit {
    #[default]
    Cost,
    Tokens,
    CostPerMtok,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpendBreakdown {
    #[default]
    Models,
    Projects,
}

impl Default for DisplaySettings {
    fn default() -> DisplaySettings {
        DisplaySettings {
            theme: Theme::System,
            language: Language::System,
            value_mode: ValueMode::Left,
            reset_format: ResetFormat::Countdown,
            panel_label: PanelLabel::Percent,
            show_spend: true,
            show_account_spend: true,
            show_trend: true,
            show_forecast: true,
            translucent: false,
            combine_accounts: false,
            hidden_windows: BTreeMap::new(),
            density: Density::Normal,
            time_format: TimeFormat::Auto,
            panel_mode: PanelMode::Headline,
            panel_indicator: PanelIndicator::Ring,
            panel_limits: Vec::new(),
            panel_position: PanelPosition::default(),
            spend_period: SpendPeriod::Last30Days,
            spend_unit: SpendUnit::Cost,
            spend_breakdown: SpendBreakdown::Models,
            starred_accounts: Vec::new(),
            collapse_unstarred: false,
            hide_on_screen_share: true,
        }
    }
}

impl DisplaySettings {
    #[must_use]
    pub fn is_hidden(&self, account_id: &str, window: &str) -> bool {
        self.hidden_windows
            .get(account_id)
            .is_some_and(|windows| windows.iter().any(|w| w == window))
    }

    pub(super) fn validate(&self) -> Result<(), SettingsError> {
        let invalid = self
            .hidden_windows
            .iter()
            .any(|(account, windows)| blank(account) || windows.iter().any(|w| blank(w)));
        if invalid {
            return Err(SettingsError::BlankHiddenWindow);
        }
        if self.starred_accounts.iter().any(|id| blank(id)) {
            return Err(SettingsError::BlankStarredAccount);
        }
        panel::validate_limits(&self.panel_limits)
    }

    pub(super) fn normalize(&mut self) {
        for windows in self.hidden_windows.values_mut() {
            panel::dedup_in_order(windows);
        }
        panel::dedup_in_order(&mut self.panel_limits);
        panel::dedup_in_order(&mut self.starred_accounts);
    }
}

fn blank(text: &str) -> bool {
    text.trim().is_empty()
}
