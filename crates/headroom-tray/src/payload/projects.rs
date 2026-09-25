use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ProjectSpend {
    #[serde(default)]
    pub project: Option<String>,
    pub cost_usd_micros: i64,
    pub total_tokens: u64,
    pub partial: bool,
    pub share_permille: u32,
    #[serde(default)]
    pub cost_per_mtok_usd_micros: Option<i64>,
    #[serde(default)]
    pub by_provider: Vec<ProjectProvider>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ProjectProvider {
    pub provider: String,
    pub provider_name: String,
    pub cost_usd_micros: i64,
    pub total_tokens: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct OtherProjects {
    pub count: u64,
    pub cost_usd_micros: i64,
    pub total_tokens: u64,
    pub partial: bool,
    pub share_permille: u32,
    #[serde(default)]
    pub cost_per_mtok_usd_micros: Option<i64>,
}
