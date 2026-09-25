use std::collections::BTreeMap;
use std::fmt;
use std::ops::RangeInclusive;

use serde::Deserialize;

pub const THRESHOLD_RANGE: RangeInclusive<u8> = 1..=50;
pub const PROVIDER_THRESHOLD_RANGE: RangeInclusive<u8> = 0..=50;
pub const DEFAULT_THRESHOLD: u8 = 10;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
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
    pub threshold_percent: u8,
    pub provider_thresholds: BTreeMap<String, u8>,
    pub quiet_hours: QuietHours,
}

impl Default for Notifications {
    fn default() -> Self {
        Self {
            almost_out: true,
            cutting_it_close: true,
            will_run_out: true,
            reset: false,
            threshold_percent: DEFAULT_THRESHOLD,
            provider_thresholds: BTreeMap::new(),
            quiet_hours: QuietHours::default(),
        }
    }
}

impl Notifications {
    #[must_use]
    pub fn threshold_for(&self, provider: &str) -> u8 {
        self.provider_thresholds
            .get(provider)
            .copied()
            .unwrap_or(self.threshold_percent)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct QuietHours {
    pub enabled: bool,
    pub from: ClockTime,
    pub to: ClockTime,
    pub allow_critical: bool,
}

impl Default for QuietHours {
    fn default() -> Self {
        Self {
            enabled: false,
            from: ClockTime::DEFAULT_FROM,
            to: ClockTime::DEFAULT_TO,
            allow_critical: true,
        }
    }
}

impl QuietHours {
    #[must_use]
    pub fn is_valid(&self) -> bool {
        !self.enabled || self.from != self.to
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(try_from = "String")]
pub struct ClockTime {
    hour: u8,
    minute: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("not a HH:MM time: {0}")]
pub struct ClockTimeError(String);

impl ClockTime {
    pub const DEFAULT_FROM: ClockTime = ClockTime {
        hour: 22,
        minute: 0,
    };
    pub const DEFAULT_TO: ClockTime = ClockTime { hour: 8, minute: 0 };

    #[must_use]
    pub fn new(hour: u8, minute: u8) -> Option<Self> {
        (hour < 24 && minute < 60).then_some(Self { hour, minute })
    }

    #[must_use]
    pub fn hour(self) -> u8 {
        self.hour
    }

    #[must_use]
    pub fn minute(self) -> u8 {
        self.minute
    }

    pub fn parse(text: &str) -> Result<Self, ClockTimeError> {
        parse_clock(text).ok_or_else(|| ClockTimeError(text.to_owned()))
    }
}

fn two_digits(part: &str) -> Option<u8> {
    (part.len() == 2 && part.bytes().all(|b| b.is_ascii_digit()))
        .then(|| part.parse().ok())
        .flatten()
}

fn parse_clock(text: &str) -> Option<ClockTime> {
    let (hour, minute) = text.split_once(':')?;
    ClockTime::new(two_digits(hour)?, two_digits(minute)?)
}

impl TryFrom<String> for ClockTime {
    type Error = ClockTimeError;

    fn try_from(text: String) -> Result<Self, ClockTimeError> {
        ClockTime::parse(&text)
    }
}

impl fmt::Display for ClockTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:02}:{:02}", self.hour, self.minute)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clock_times_need_two_digits_each() {
        assert_eq!(ClockTime::parse("07:30").unwrap().to_string(), "07:30");
        assert_eq!(
            ClockTime::parse("23:59").unwrap(),
            ClockTime::new(23, 59).unwrap()
        );
        for bad in ["7:30", "24:00", "12:60", "12-00", "", "ab:cd", "123:00"] {
            assert!(ClockTime::parse(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn quiet_hours_need_a_range_only_when_enabled() {
        let mut quiet = QuietHours {
            to: ClockTime::DEFAULT_FROM,
            ..QuietHours::default()
        };
        assert!(quiet.is_valid());
        quiet.enabled = true;
        assert!(!quiet.is_valid());
        quiet.to = ClockTime::DEFAULT_TO;
        assert!(quiet.is_valid());
    }

    #[test]
    fn provider_thresholds_override_the_general_one() {
        let mut notifications = Notifications::default();
        notifications
            .provider_thresholds
            .insert("copilot".into(), 0);
        assert_eq!(notifications.threshold_for("copilot"), 0);
        assert_eq!(notifications.threshold_for("claude"), DEFAULT_THRESHOLD);
    }
}
