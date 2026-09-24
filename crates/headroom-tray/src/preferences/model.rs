use std::collections::BTreeMap;

use serde::Deserialize;
use serde_json::{Map, Value};

use crate::payload::{Language, ResetFormat, Theme, ValueMode};

pub const MIN_REFRESH_SECS: u32 = 60;
pub const MAX_REFRESH_SECS: u32 = 3600;
pub const DEFAULT_REFRESH_SECS: u32 = 300;

#[derive(Debug, thiserror::Error)]
pub enum SettingsError {
    #[error("unreadable settings from the Headroom service: {0}")]
    Json(#[from] serde_json::Error),
    #[error("unexpected settings from the Headroom service")]
    NotAnObject,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub refresh_interval_secs: u32,
    pub notifications: Notifications,
    pub headline: Headline,
    pub reduced_motion: bool,
    pub display: DisplaySettings,
    pub updates: Updates,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            refresh_interval_secs: DEFAULT_REFRESH_SECS,
            notifications: Notifications::default(),
            headline: Headline::Auto,
            reduced_motion: false,
            display: DisplaySettings::default(),
            updates: Updates::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(default)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "each flag is an independent notification toggle"
)]
pub struct Notifications {
    pub almost_out: bool,
    pub cutting_it_close: bool,
    pub will_run_out: bool,
    pub reset: bool,
}

impl Default for Notifications {
    fn default() -> Self {
        Self {
            almost_out: true,
            cutting_it_close: true,
            will_run_out: true,
            reset: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct Updates {
    pub check: bool,
}

impl Default for Updates {
    fn default() -> Self {
        Self { check: true }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
#[serde(from = "RawHeadline")]
pub enum Headline {
    #[default]
    Auto,
    Pinned {
        account_id: String,
        window: String,
    },
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct RawHeadline {
    mode: String,
    account_id: Option<String>,
    window: Option<String>,
}

impl From<RawHeadline> for Headline {
    fn from(raw: RawHeadline) -> Self {
        match (raw.mode.as_str(), raw.account_id, raw.window) {
            ("pinned", Some(account_id), Some(window))
                if !account_id.is_empty() && !window.is_empty() =>
            {
                Headline::Pinned { account_id, window }
            }
            _ => Headline::Auto,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "each flag is an independent user toggle from the daemon"
)]
pub struct DisplaySettings {
    pub theme: Theme,
    pub language: Language,
    pub value_mode: ValueMode,
    pub reset_format: ResetFormat,
    pub show_spend: bool,
    pub show_account_spend: bool,
    pub show_trend: bool,
    pub show_forecast: bool,
    pub combine_accounts: bool,
    pub hidden_windows: BTreeMap<String, Vec<String>>,
}

impl Default for DisplaySettings {
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
            combine_accounts: false,
            hidden_windows: BTreeMap::new(),
        }
    }
}

impl DisplaySettings {
    #[must_use]
    pub fn is_window_hidden(&self, account_id: &str, window_id: &str) -> bool {
        self.hidden_windows
            .get(account_id)
            .is_some_and(|windows| windows.iter().any(|id| id == window_id))
    }

    #[must_use]
    pub fn hidden_windows_after(
        &self,
        account_id: &str,
        window_id: &str,
        hidden: bool,
    ) -> Vec<String> {
        let mut windows: Vec<String> = self
            .hidden_windows
            .get(account_id)
            .map(|windows| {
                windows
                    .iter()
                    .filter(|id| *id != window_id)
                    .cloned()
                    .collect()
            })
            .unwrap_or_default();
        if hidden {
            windows.push(window_id.to_owned());
        }
        windows
    }
}

pub fn decode_settings(json: &str) -> Result<Value, SettingsError> {
    let raw: Value = serde_json::from_str(json)?;
    if raw.is_object() {
        Ok(raw)
    } else {
        Err(SettingsError::NotAnObject)
    }
}

pub fn settings_from(raw: &Value) -> Result<Settings, SettingsError> {
    Ok(Settings::deserialize(raw)?)
}

pub fn merge_patch(target: &mut Value, patch: &Value) {
    let Value::Object(changes) = patch else {
        target.clone_from(patch);
        return;
    };
    if !target.is_object() {
        *target = Value::Object(Map::new());
    }
    let Value::Object(fields) = target else {
        return;
    };
    for (key, value) in changes {
        if value.is_null() {
            fields.remove(key);
        } else {
            merge_patch(fields.entry(key.clone()).or_insert(Value::Null), value);
        }
    }
}

#[cfg(test)]
#[path = "model_tests.rs"]
mod tests;
