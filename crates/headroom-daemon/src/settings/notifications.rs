use std::collections::BTreeMap;
use std::fmt;
use std::ops::RangeInclusive;

use serde::{Deserialize, Serialize};

use crate::error::SettingsError;

pub const THRESHOLD_PERCENT_RANGE: RangeInclusive<u8> = 1..=50;
pub const PROVIDER_THRESHOLD_RANGE: RangeInclusive<u8> = 0..=50;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
    pub threshold_percent: u8,
    pub provider_thresholds: BTreeMap<String, u8>,
    pub quiet_hours: QuietHours,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct QuietHours {
    pub enabled: bool,
    pub from: ClockTime,
    pub to: ClockTime,
    pub allow_critical: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct ClockTime {
    hour: u8,
    minute: u8,
}

impl Default for NotificationSettings {
    fn default() -> NotificationSettings {
        NotificationSettings {
            almost_out: true,
            cutting_it_close: true,
            will_run_out: true,
            reset: false,
            threshold_percent: 10,
            provider_thresholds: BTreeMap::new(),
            quiet_hours: QuietHours::default(),
        }
    }
}

impl Default for QuietHours {
    fn default() -> QuietHours {
        QuietHours {
            enabled: false,
            from: ClockTime::at(22, 0),
            to: ClockTime::at(8, 0),
            allow_critical: true,
        }
    }
}

impl NotificationSettings {
    pub(super) fn validate(&self) -> Result<(), SettingsError> {
        if !THRESHOLD_PERCENT_RANGE.contains(&self.threshold_percent) {
            return Err(SettingsError::ThresholdPercent(self.threshold_percent));
        }
        for (provider, value) in &self.provider_thresholds {
            if provider.trim().is_empty() {
                return Err(SettingsError::BlankProviderThreshold);
            }
            if !PROVIDER_THRESHOLD_RANGE.contains(value) {
                return Err(SettingsError::ProviderThreshold {
                    provider: provider.clone(),
                    value: *value,
                });
            }
        }
        if self.quiet_hours.enabled && self.quiet_hours.from == self.quiet_hours.to {
            return Err(SettingsError::EmptyQuietHours);
        }
        Ok(())
    }
}

impl ClockTime {
    const fn at(hour: u8, minute: u8) -> ClockTime {
        ClockTime { hour, minute }
    }
}

impl TryFrom<String> for ClockTime {
    type Error = SettingsError;

    fn try_from(text: String) -> Result<ClockTime, SettingsError> {
        parse_clock(&text).ok_or(SettingsError::ClockTime(text))
    }
}

fn parse_clock(text: &str) -> Option<ClockTime> {
    let (hour, minute) = text.split_once(':')?;
    let two_digits = |part: &str| part.len() == 2 && part.bytes().all(|b| b.is_ascii_digit());
    if !two_digits(hour) || !two_digits(minute) {
        return None;
    }
    let (hour, minute) = (hour.parse::<u8>().ok()?, minute.parse::<u8>().ok()?);
    (hour < 24 && minute < 60).then_some(ClockTime { hour, minute })
}

impl From<ClockTime> for String {
    fn from(time: ClockTime) -> String {
        time.to_string()
    }
}

impl fmt::Display for ClockTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:02}:{:02}", self.hour, self.minute)
    }
}
