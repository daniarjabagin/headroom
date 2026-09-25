use serde::{Deserialize, Serialize};

use crate::error::SettingsError;

pub const MAX_SHORTCUT_CHARS: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct UpdateSettings {
    pub check: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct StatusPageSettings {
    pub enabled: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ShortcutSettings {
    pub open: String,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct LoggingSettings {
    pub level: LogLevel,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LogLevel {
    Error,
    Warn,
    #[default]
    Info,
    Debug,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct OnboardingSettings {
    pub completed: bool,
}

impl Default for UpdateSettings {
    fn default() -> UpdateSettings {
        UpdateSettings { check: true }
    }
}

impl ShortcutSettings {
    pub(super) fn validate(&self) -> Result<(), SettingsError> {
        if self.open.chars().count() > MAX_SHORTCUT_CHARS {
            return Err(SettingsError::ShortcutTooLong(MAX_SHORTCUT_CHARS));
        }
        if self.open.is_empty() || is_accelerator(&self.open) {
            Ok(())
        } else {
            Err(SettingsError::InvalidShortcut(self.open.clone()))
        }
    }
}

fn is_accelerator(text: &str) -> bool {
    let mut rest = text;
    while let Some(tail) = rest.strip_prefix('<') {
        let Some((modifier, after)) = tail.split_once('>') else {
            return false;
        };
        if modifier.is_empty() || !modifier.chars().all(|c| c.is_ascii_alphabetic()) {
            return false;
        }
        rest = after;
    }
    !rest.is_empty() && rest.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}
