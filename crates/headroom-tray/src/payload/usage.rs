use serde::Deserialize;

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
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct OtherModels {
    pub count: u64,
    pub total_tokens: u64,
    pub cost_usd_micros: i64,
    pub partial: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
pub struct Spend {
    pub today: PeriodSpend,
    pub yesterday: PeriodSpend,
    pub last_30_days: PeriodSpend,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
pub struct PeriodSpend {
    pub cost_usd_micros: i64,
    pub total_tokens: u64,
    pub partial: bool,
    pub by_provider: Vec<ProviderSpend>,
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
}
