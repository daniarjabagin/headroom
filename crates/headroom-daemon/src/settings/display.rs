use std::collections::{BTreeMap, HashSet};

use serde::{Deserialize, Serialize};

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
    pub hidden_windows: BTreeMap<String, Vec<String>>,
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
pub enum PanelLabel {
    #[default]
    Percent,
    Window,
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
            hidden_windows: BTreeMap::new(),
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
        let blank = |text: &String| text.trim().is_empty();
        let invalid = self
            .hidden_windows
            .iter()
            .any(|(account, windows)| blank(account) || windows.iter().any(blank));
        if invalid {
            return Err(SettingsError::BlankHiddenWindow);
        }
        Ok(())
    }

    pub(super) fn normalize(&mut self) {
        for windows in self.hidden_windows.values_mut() {
            let mut seen = HashSet::new();
            windows.retain(|window| seen.insert(window.clone()));
        }
    }
}
