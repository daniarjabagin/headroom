use serde::Deserialize;
use serde_json::{Map, Value};

pub use super::notify::{ClockTime, Notifications, QuietHours};
pub use crate::payload::Display as DisplaySettings;

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
    pub adaptive_refresh: bool,
    pub notifications: Notifications,
    pub headline: Headline,
    pub reduced_motion: bool,
    pub display: DisplaySettings,
    pub updates: Updates,
    pub status_pages: StatusPages,
    pub shortcuts: Shortcuts,
    pub logging: Logging,
    pub onboarding: Onboarding,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            refresh_interval_secs: DEFAULT_REFRESH_SECS,
            adaptive_refresh: true,
            notifications: Notifications::default(),
            headline: Headline::Auto,
            reduced_motion: false,
            display: DisplaySettings::default(),
            updates: Updates::default(),
            status_pages: StatusPages::default(),
            shortcuts: Shortcuts::default(),
            logging: Logging::default(),
            onboarding: Onboarding::default(),
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(default)]
pub struct StatusPages {
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
#[serde(default)]
pub struct Shortcuts {
    pub open: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(default)]
pub struct Logging {
    pub level: LogLevel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LogLevel {
    Error,
    Warn,
    Debug,
    #[default]
    #[serde(other)]
    Info,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(default)]
pub struct Onboarding {
    pub completed: bool,
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
