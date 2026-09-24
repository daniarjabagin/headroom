mod dismissed;
mod display;
mod migrate;
mod patch;

use std::collections::BTreeSet;
use std::ops::RangeInclusive;

use jiff::SignedDuration;
use serde::{Deserialize, Serialize};

use crate::error::SettingsError;

pub use display::{DisplaySettings, Language, PanelLabel, ResetFormat, Theme, ValueMode};

pub const REFRESH_INTERVAL_RANGE: RangeInclusive<u64> = 60..=3_600;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Settings {
    pub refresh_interval_secs: u64,
    pub notifications: NotificationSettings,
    pub headline: HeadlineMode,
    pub reduced_motion: bool,
    pub display: DisplaySettings,
    pub dismissed_accounts: BTreeSet<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "each flag is an independent user toggle"
)]
pub struct NotificationSettings {
    pub almost_out: bool,
    pub cutting_it_close: bool,
    pub will_run_out: bool,
    pub reset: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case", deny_unknown_fields)]
pub enum HeadlineMode {
    Auto {},
    Pinned { account_id: String, window: String },
}

impl Default for Settings {
    fn default() -> Settings {
        Settings {
            refresh_interval_secs: 300,
            notifications: NotificationSettings::default(),
            headline: HeadlineMode::Auto {},
            reduced_motion: false,
            display: DisplaySettings::default(),
            dismissed_accounts: BTreeSet::new(),
        }
    }
}

impl Default for HeadlineMode {
    fn default() -> HeadlineMode {
        HeadlineMode::Auto {}
    }
}

impl Default for NotificationSettings {
    fn default() -> NotificationSettings {
        NotificationSettings {
            almost_out: true,
            cutting_it_close: true,
            will_run_out: true,
            reset: false,
        }
    }
}

impl Settings {
    pub fn parse(json: &str) -> Result<Settings, SettingsError> {
        let settings: Settings = serde_json::from_str(json)?;
        settings.validated()
    }

    pub fn from_stored(json: &str) -> Result<Settings, serde_json::Error> {
        let mut settings: Settings = serde_json::from_value(migrate::upgrade(json)?)?;
        settings.display.normalize();
        Ok(settings)
    }

    fn validated(mut self) -> Result<Settings, SettingsError> {
        if !REFRESH_INTERVAL_RANGE.contains(&self.refresh_interval_secs) {
            return Err(SettingsError::RefreshInterval(self.refresh_interval_secs));
        }
        if let HeadlineMode::Pinned { account_id, window } = &self.headline
            && (account_id.trim().is_empty() || window.trim().is_empty())
        {
            return Err(SettingsError::EmptyHeadlineTarget);
        }
        self.display.validate()?;
        self.display.normalize();
        self.validate_dismissed()?;
        Ok(self)
    }

    #[must_use]
    pub fn refresh_interval(&self) -> SignedDuration {
        let secs = i64::try_from(self.refresh_interval_secs).unwrap_or(i64::MAX);
        SignedDuration::from_secs(secs)
    }
}

#[cfg(test)]
mod tests;
