use jiff::Timestamp;
use serde::Deserialize;

use super::Tone;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ProviderStatus {
    pub provider: String,
    pub indicator: StatusIndicator,
    pub tone: Tone,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub stage: Option<String>,
    #[serde(default)]
    pub started_at: Option<Timestamp>,
    pub url: String,
}

impl ProviderStatus {
    #[must_use]
    pub fn is_clear(&self) -> bool {
        self.indicator == StatusIndicator::None
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StatusIndicator {
    Minor,
    Major,
    Critical,
    Maintenance,
    #[serde(other)]
    None,
}
