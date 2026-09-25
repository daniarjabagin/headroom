use headroom_core::account::ProviderId;
use headroom_core::tokens::TokenCounts;
use jiff::civil::Date;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UsageView {
    pub provider: ProviderId,
    pub provider_name: String,
    pub usage_home: String,
    pub today: TotalsView,
    pub yesterday: TotalsView,
    pub last_30_days: TotalsView,
    pub daily: Vec<DailyView>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TotalsView {
    pub tokens: TokensView,
    pub cost_usd_micros: i64,
    pub partial: bool,
    pub unpriced_tokens: u64,
    pub unpriced_models: Vec<String>,
    pub models: Vec<ModelView>,
    pub models_other: Option<OtherModelsView>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokensView {
    pub input: u64,
    pub cache_read: u64,
    pub cache_write: u64,
    pub output: u64,
    pub reasoning: u64,
    pub total: u64,
}

impl TokensView {
    #[must_use]
    pub fn of(tokens: &TokenCounts) -> TokensView {
        TokensView {
            input: tokens.input.0,
            cache_read: tokens.cache_read.0,
            cache_write: tokens.cache_write().0,
            output: tokens.output.0,
            reasoning: tokens.reasoning.0,
            total: tokens.total().0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DailyView {
    pub date: Date,
    pub total_tokens: u64,
    pub cost_usd_micros: i64,
    pub partial: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelView {
    pub model: String,
    pub total_tokens: u64,
    pub cost_usd_micros: i64,
    pub partial: bool,
    pub cost_per_mtok_usd_micros: Option<i64>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OtherModelsView {
    pub count: usize,
    pub total_tokens: u64,
    pub cost_usd_micros: i64,
    pub partial: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpendView {
    pub today: PeriodSpendView,
    pub yesterday: PeriodSpendView,
    #[serde(default)]
    pub last_7_days: PeriodSpendView,
    pub last_30_days: PeriodSpendView,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PeriodSpendView {
    pub cost_usd_micros: i64,
    pub total_tokens: u64,
    pub partial: bool,
    pub cost_per_mtok_usd_micros: Option<i64>,
    pub by_provider: Vec<ProviderSpendView>,
    #[serde(default)]
    pub projects: Vec<ProjectSpendView>,
    pub projects_other: Option<OtherProjectsView>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderSpendView {
    pub provider: ProviderId,
    pub provider_name: String,
    pub cost_usd_micros: i64,
    pub total_tokens: u64,
    pub partial: bool,
    pub cost_per_mtok_usd_micros: Option<i64>,
    pub models: Vec<ModelView>,
    pub models_other: Option<OtherModelsView>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectSpendView {
    pub project: Option<String>,
    pub cost_usd_micros: i64,
    pub total_tokens: u64,
    pub partial: bool,
    pub share_permille: u32,
    pub by_provider: Vec<ProjectProviderView>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectProviderView {
    pub provider: ProviderId,
    pub provider_name: String,
    pub cost_usd_micros: i64,
    pub total_tokens: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OtherProjectsView {
    pub count: usize,
    pub cost_usd_micros: i64,
    pub total_tokens: u64,
    pub partial: bool,
    pub share_permille: u32,
}
