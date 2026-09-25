mod display;
mod migrate;
mod notifications;
mod panel;
mod patch;
mod sections;

use std::ops::RangeInclusive;

use jiff::SignedDuration;
use serde::{Deserialize, Serialize};

use crate::error::SettingsError;

pub use display::{
    Density, DisplaySettings, Language, ResetFormat, SpendBreakdown, SpendPeriod, SpendUnit, Theme,
    TimeFormat, ValueMode,
};
pub use notifications::{ClockTime, NotificationSettings, QuietHours};
pub use panel::{PanelBox, PanelIndicator, PanelLabel, PanelLimit, PanelMode, PanelPosition};
pub use sections::{
    LogLevel, LoggingSettings, OnboardingSettings, ShortcutSettings, StatusPageSettings,
    UpdateSettings,
};

pub const REFRESH_INTERVAL_RANGE: RangeInclusive<u64> = 60..=3_600;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Settings {
    pub refresh_interval_secs: u64,
    pub adaptive_refresh: bool,
    pub notifications: NotificationSettings,
    pub headline: HeadlineMode,
    pub reduced_motion: bool,
    pub display: DisplaySettings,
    pub updates: UpdateSettings,
    pub status_pages: StatusPageSettings,
    pub shortcuts: ShortcutSettings,
    pub logging: LoggingSettings,
    pub onboarding: OnboardingSettings,
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
            adaptive_refresh: true,
            notifications: NotificationSettings::default(),
            headline: HeadlineMode::Auto {},
            reduced_motion: false,
            display: DisplaySettings::default(),
            updates: UpdateSettings::default(),
            status_pages: StatusPageSettings::default(),
            shortcuts: ShortcutSettings::default(),
            logging: LoggingSettings::default(),
            onboarding: OnboardingSettings::default(),
        }
    }
}

impl Default for HeadlineMode {
    fn default() -> HeadlineMode {
        HeadlineMode::Auto {}
    }
}

impl Settings {
    pub fn parse(json: &str) -> Result<Settings, SettingsError> {
        let mut value: serde_json::Value = serde_json::from_str(json)?;
        migrate::drop_daemon_managed(&mut value);
        let settings: Settings = serde_json::from_value(value)?;
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
        self.notifications.validate()?;
        self.shortcuts.validate()?;
        self.display.normalize();
        Ok(self)
    }

    #[must_use]
    pub fn reset(&self) -> Settings {
        Settings {
            onboarding: self.onboarding,
            ..Settings::default()
        }
    }

    #[must_use]
    pub fn refresh_interval(&self) -> SignedDuration {
        let secs = i64::try_from(self.refresh_interval_secs).unwrap_or(i64::MAX);
        SignedDuration::from_secs(secs)
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
#[path = "options_tests.rs"]
mod options_tests;
