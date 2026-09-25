use headroom_core::account::{CredentialOwner, ProviderId};
use headroom_core::pace::{Severity, Tone};
use headroom_core::tokens::TokenCounts;
use jiff::Timestamp;
use jiff::civil::Date;
use serde::{Deserialize, Serialize};

use crate::settings::DisplaySettings;
use crate::update::InstallKind;

pub const STATE_VERSION: u32 = 1;
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StatePayload {
    pub version: u32,
    pub app_version: Option<String>,
    pub generated_at: Timestamp,
    pub next_refresh_at: Option<Timestamp>,
    pub last_success_at: Option<Timestamp>,
    pub offline: bool,
    pub update: Option<UpdateView>,
    #[serde(default)]
    pub update_check: Option<UpdateCheckView>,
    pub display: DisplaySettings,
    pub headline: Option<Headline>,
    #[serde(default)]
    pub panel_items: Vec<PanelItem>,
    #[serde(default)]
    pub panel_tone: Option<Tone>,
    pub accounts: Vec<AccountView>,
    #[serde(default)]
    pub combined: Vec<CombinedView>,
    pub usage: Vec<UsageView>,
    pub spend: SpendView,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateView {
    pub version: String,
    pub url: String,
    pub published_at: Timestamp,
    pub install: InstallKind,
    pub command: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateCheckView {
    pub checked_at: Option<Timestamp>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Headline {
    pub account_id: String,
    pub provider: ProviderId,
    pub provider_name: String,
    pub account_label: Option<String>,
    pub window: String,
    pub window_label: String,
    pub used_percent: f64,
    pub remaining_percent: f64,
    pub tone: Tone,
    #[serde(default)]
    pub combined: bool,
    #[serde(default = "single_account")]
    pub account_count: usize,
}

fn single_account() -> usize {
    1
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PanelItem {
    #[serde(flatten)]
    pub headline: Headline,
    pub value_percent: f64,
    pub even_pace_percent: Option<f64>,
    pub logo: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AccountView {
    pub id: String,
    pub provider: ProviderId,
    pub provider_name: String,
    pub label: Option<String>,
    pub email: Option<String>,
    pub plan: Option<String>,
    pub hidden: bool,
    pub owner: CredentialOwner,
    pub status: AccountStatus,
    pub error: Option<AccountError>,
    #[serde(default)]
    pub recovery: Option<Recovery>,
    pub updated_at: Option<Timestamp>,
    pub source: Option<DataSource>,
    pub windows: Vec<WindowView>,
    pub balances: Vec<BalanceView>,
    pub notices: Vec<NoticeView>,
    pub usage_home: String,
    #[serde(default)]
    pub refresh: Option<RefreshView>,
    #[serde(default)]
    pub collapsed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RefreshView {
    pub mode: RefreshMode,
    pub interval_secs: i64,
    pub next_at: Option<Timestamp>,
    pub reason: RefreshReason,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RefreshMode {
    Live,
    Idle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RefreshReason {
    Activity,
    Schedule,
    Backoff,
    Hold,
}

impl AccountView {
    #[must_use]
    pub fn display_label(&self) -> String {
        self.label
            .clone()
            .or_else(|| self.email.clone())
            .unwrap_or_else(|| self.provider_name.clone())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccountStatus {
    Fresh,
    Stale,
    Refreshing,
    Error,
    SignedOut,
    NoSubscription,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DataSource {
    Live,
    LocalLog,
    Cache,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountError {
    pub kind: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum Recovery {
    Retry,
    SignIn { account_id: String },
    CliLogin { command: String },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WindowView {
    pub id: String,
    pub label: String,
    pub used_percent: f64,
    pub remaining_percent: f64,
    pub resets_at: Option<Timestamp>,
    pub period_seconds: Option<i64>,
    pub tone: Tone,
    pub pace: PaceView,
    pub hidden: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PaceView {
    pub severity: Severity,
    pub even_pace_percent: Option<f64>,
    pub projected_percent: Option<f64>,
    pub spare_percent: Option<f64>,
    pub runs_out_at: Option<Timestamp>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CombinedView {
    pub provider: ProviderId,
    pub provider_name: String,
    pub account_ids: Vec<String>,
    pub accounts: Vec<CombinedAccountView>,
    pub windows: Vec<CombinedWindowView>,
    #[serde(default)]
    pub collapsed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CombinedAccountView {
    pub account_id: String,
    pub label: String,
    pub plan: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CombinedWindowView {
    pub id: String,
    pub label: String,
    pub capacity_percent: u32,
    pub remaining_percent: f64,
    pub used_percent: f64,
    pub resets_at: Option<Timestamp>,
    pub tone: Tone,
    pub pace: PaceView,
    pub segments: Vec<SegmentView>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SegmentView {
    pub account_id: String,
    pub label: String,
    pub remaining_percent: f64,
    pub used_percent: f64,
    pub resets_at: Option<Timestamp>,
    pub tone: Tone,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BalanceView {
    pub id: String,
    pub label: String,
    #[serde(flatten)]
    pub amount: BalanceAmountView,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum BalanceAmountView {
    Usd { usd_micros: i64 },
    Money { currency: String, micros: i64 },
    Count { value: u64, unit: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NoticeView {
    pub tone: Tone,
    pub text: String,
}

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

#[cfg(test)]
#[path = "payload_tests.rs"]
mod tests;
