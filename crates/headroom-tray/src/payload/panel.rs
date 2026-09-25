use serde::Deserialize;

use super::{Tone, ValueMode};

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Headline {
    #[serde(default)]
    pub account_id: Option<String>,
    pub provider: String,
    pub provider_name: String,
    #[serde(default)]
    pub account_label: Option<String>,
    #[serde(default)]
    pub combined: bool,
    #[serde(default)]
    pub account_count: Option<u32>,
    pub window: String,
    pub window_label: String,
    pub used_percent: f64,
    pub remaining_percent: f64,
    pub tone: Tone,
}

impl Headline {
    #[must_use]
    pub fn value_percent(&self, mode: ValueMode) -> f64 {
        match mode {
            ValueMode::Left => self.remaining_percent,
            ValueMode::Used => self.used_percent,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct PanelItem {
    #[serde(flatten)]
    pub headline: Headline,
    pub value_percent: f64,
    #[serde(default)]
    pub even_pace_percent: Option<f64>,
    #[serde(default)]
    pub logo: Option<String>,
}

impl PanelItem {
    #[must_use]
    pub fn from_headline(headline: &Headline, mode: ValueMode) -> Self {
        Self {
            value_percent: headline.value_percent(mode),
            even_pace_percent: None,
            logo: None,
            headline: headline.clone(),
        }
    }

    #[must_use]
    pub fn logo(&self) -> &str {
        self.logo.as_deref().unwrap_or(&self.headline.provider)
    }
}
