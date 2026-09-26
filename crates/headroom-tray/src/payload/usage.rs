use serde::Deserialize;

use super::SpendPeriod;
use super::projects::{OtherProjects, ProjectSpend};

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Usage {
    pub provider: String,
    pub provider_name: String,
    pub usage_home: String,
    pub today: Totals,
    pub yesterday: Totals,
    pub last_30_days: Totals,
    pub daily: Vec<Daily>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Totals {
    pub tokens: Tokens,
    pub cost_usd_micros: i64,
    pub partial: bool,
    pub models: Vec<ModelUsage>,
    pub models_other: Option<OtherModels>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Tokens {
    #[serde(default)]
    pub input: u64,
    #[serde(default)]
    pub cache_read: u64,
    #[serde(default)]
    pub cache_write: u64,
    #[serde(default)]
    pub output: u64,
    #[serde(default)]
    pub reasoning: u64,
    pub total: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Daily {
    pub date: jiff::civil::Date,
    pub total_tokens: u64,
    pub cost_usd_micros: i64,
    pub partial: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ModelUsage {
    pub model: String,
    pub total_tokens: u64,
    pub cost_usd_micros: i64,
    pub partial: bool,
    #[serde(default)]
    pub cost_per_mtok_usd_micros: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct OtherModels {
    pub count: u64,
    pub total_tokens: u64,
    pub cost_usd_micros: i64,
    pub partial: bool,
    #[serde(default)]
    pub cost_per_mtok_usd_micros: Option<i64>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
pub struct Spend {
    pub today: PeriodSpend,
    pub yesterday: PeriodSpend,
    #[serde(default)]
    pub last_7_days: Option<PeriodSpend>,
    pub last_30_days: PeriodSpend,
}

impl Spend {
    #[must_use]
    pub fn period(&self, period: SpendPeriod) -> Option<&PeriodSpend> {
        match period {
            SpendPeriod::Today => Some(&self.today),
            SpendPeriod::Yesterday => Some(&self.yesterday),
            SpendPeriod::Last7Days => self.last_7_days.as_ref(),
            SpendPeriod::Last30Days => Some(&self.last_30_days),
        }
    }

    #[must_use]
    pub fn has_period(&self, period: SpendPeriod) -> bool {
        self.period(period).is_some()
    }

    #[must_use]
    pub fn has_projects(&self) -> bool {
        self.today.projects.is_some()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
pub struct PeriodSpend {
    pub cost_usd_micros: i64,
    pub total_tokens: u64,
    pub partial: bool,
    pub by_provider: Vec<ProviderSpend>,
    #[serde(default)]
    pub cost_per_mtok_usd_micros: Option<i64>,
    #[serde(default)]
    pub projects: Option<Vec<ProjectSpend>>,
    #[serde(default)]
    pub projects_other: Option<OtherProjects>,
    #[serde(default)]
    pub models: Option<Vec<ProviderModelUsage>>,
    #[serde(default)]
    pub models_other: Option<OtherModels>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ProviderModelUsage {
    pub provider: String,
    pub provider_name: String,
    #[serde(flatten)]
    pub usage: ModelUsage,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ProviderSpend {
    pub provider: String,
    pub provider_name: String,
    pub cost_usd_micros: i64,
    pub total_tokens: u64,
    pub partial: bool,
    pub models: Vec<ModelUsage>,
    pub models_other: Option<OtherModels>,
    #[serde(default)]
    pub cost_per_mtok_usd_micros: Option<i64>,
}
