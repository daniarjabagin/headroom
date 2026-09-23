use std::ops::RangeInclusive;

use jiff::SignedDuration;
use serde::{Deserialize, Serialize};

use crate::error::SettingsError;

pub const REFRESH_INTERVAL_RANGE: RangeInclusive<u64> = 60..=3_600;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub refresh_interval_secs: u64,
    pub notifications: NotificationSettings,
    pub headline: HeadlineMode,
    pub show_usage: bool,
    pub reduced_motion: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
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

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum HeadlineMode {
    #[default]
    Auto,
    Pinned {
        account_id: String,
        window: String,
    },
}

impl Default for Settings {
    fn default() -> Settings {
        Settings {
            refresh_interval_secs: 300,
            notifications: NotificationSettings::default(),
            headline: HeadlineMode::Auto,
            show_usage: true,
            reduced_motion: false,
        }
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
        settings.validate()?;
        Ok(settings)
    }

    pub fn validate(&self) -> Result<(), SettingsError> {
        if !REFRESH_INTERVAL_RANGE.contains(&self.refresh_interval_secs) {
            return Err(SettingsError::RefreshInterval(self.refresh_interval_secs));
        }
        match &self.headline {
            HeadlineMode::Pinned { account_id, window }
                if account_id.trim().is_empty() || window.trim().is_empty() =>
            {
                Err(SettingsError::EmptyHeadlineTarget)
            }
            _ => Ok(()),
        }
    }

    #[must_use]
    pub fn refresh_interval(&self) -> SignedDuration {
        let secs = i64::try_from(self.refresh_interval_secs).unwrap_or(i64::MAX);
        SignedDuration::from_secs(secs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_the_spec() {
        let settings = Settings::default();
        assert_eq!(settings.refresh_interval_secs, 300);
        assert!(settings.notifications.almost_out);
        assert!(settings.notifications.cutting_it_close);
        assert!(settings.notifications.will_run_out);
        assert!(!settings.notifications.reset);
        assert_eq!(settings.headline, HeadlineMode::Auto);
        assert!(settings.show_usage);
        assert!(!settings.reduced_motion);
    }

    #[test]
    fn missing_fields_take_defaults() {
        let settings = Settings::parse(r#"{"notifications":{"reset":true}}"#).unwrap();
        assert!(settings.notifications.reset);
        assert!(settings.notifications.almost_out);
        assert_eq!(settings.refresh_interval_secs, 300);
    }

    #[test]
    fn serializes_headline_with_mode_tag() {
        let json = serde_json::to_value(Settings::default()).unwrap();
        assert_eq!(json["headline"], serde_json::json!({ "mode": "auto" }));
        let pinned = Settings::parse(
            r#"{"headline":{"mode":"pinned","account_id":"codex:abc","window":"session"}}"#,
        )
        .unwrap();
        assert_eq!(
            pinned.headline,
            HeadlineMode::Pinned {
                account_id: "codex:abc".into(),
                window: "session".into()
            }
        );
    }

    #[test]
    fn rejects_out_of_range_interval() {
        for value in [0, 59, 3_601] {
            let json = format!(r#"{{"refresh_interval_secs":{value}}}"#);
            assert!(matches!(
                Settings::parse(&json),
                Err(SettingsError::RefreshInterval(v)) if v == value
            ));
        }
        assert!(Settings::parse(r#"{"refresh_interval_secs":60}"#).is_ok());
        assert!(Settings::parse(r#"{"refresh_interval_secs":3600}"#).is_ok());
    }

    #[test]
    fn rejects_empty_pinned_target() {
        let json = r#"{"headline":{"mode":"pinned","account_id":" ","window":"session"}}"#;
        assert!(matches!(
            Settings::parse(json),
            Err(SettingsError::EmptyHeadlineTarget)
        ));
    }

    #[test]
    fn rejects_malformed_json_and_wrong_types() {
        assert!(matches!(Settings::parse("{"), Err(SettingsError::Json(_))));
        assert!(matches!(
            Settings::parse(r#"{"show_usage":"yes"}"#),
            Err(SettingsError::Json(_))
        ));
        assert!(matches!(
            Settings::parse(r#"{"headline":{"mode":"loudest"}}"#),
            Err(SettingsError::Json(_))
        ));
    }
}
