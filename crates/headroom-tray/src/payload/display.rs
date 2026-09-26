use std::collections::BTreeMap;

use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "each flag is an independent user toggle from the daemon"
)]
pub struct Display {
    pub theme: Theme,
    pub language: Language,
    pub value_mode: ValueMode,
    pub reset_format: ResetFormat,
    pub show_spend: bool,
    pub show_account_spend: bool,
    pub show_trend: bool,
    pub show_forecast: bool,
    pub show_breakdown: bool,
    pub translucent: bool,
    pub combine_accounts: bool,
    pub hidden_windows: BTreeMap<String, Vec<String>>,
    pub density: Density,
    pub time_format: TimeFormat,
    pub panel_mode: PanelMode,
    pub panel_indicator: PanelIndicator,
    pub panel_label: PanelLabel,
    pub panel_limits: Vec<PanelLimit>,
    pub panel_position: PanelPosition,
    pub spend_period: SpendPeriod,
    pub spend_unit: SpendUnit,
    pub spend_breakdown: SpendBreakdown,
    pub starred_accounts: Vec<String>,
    pub collapse_unstarred: bool,
    pub hide_on_screen_share: bool,
}

impl Default for Display {
    fn default() -> Self {
        Self {
            theme: Theme::System,
            language: Language::System,
            value_mode: ValueMode::Left,
            reset_format: ResetFormat::Countdown,
            show_spend: true,
            show_account_spend: true,
            show_trend: true,
            show_forecast: true,
            show_breakdown: true,
            translucent: false,
            combine_accounts: false,
            hidden_windows: BTreeMap::new(),
            density: Density::Normal,
            time_format: TimeFormat::Auto,
            panel_mode: PanelMode::Headline,
            panel_indicator: PanelIndicator::Ring,
            panel_label: PanelLabel::Percent,
            panel_limits: Vec::new(),
            panel_position: PanelPosition::default(),
            spend_period: SpendPeriod::Last30Days,
            spend_unit: SpendUnit::Cost,
            spend_breakdown: SpendBreakdown::Models,
            starred_accounts: Vec::new(),
            collapse_unstarred: false,
            hide_on_screen_share: true,
        }
    }
}

impl Display {
    #[must_use]
    pub fn is_starred(&self, account_id: &str) -> bool {
        self.starred_accounts.iter().any(|id| id == account_id)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Theme {
    Light,
    Dark,
    #[serde(other)]
    System,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Language {
    En,
    Ru,
    #[serde(other)]
    System,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValueMode {
    Used,
    #[serde(other)]
    Left,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResetFormat {
    Exact,
    #[serde(other)]
    Countdown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Density {
    Compact,
    #[serde(other)]
    Normal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum TimeFormat {
    #[serde(rename = "12h")]
    H12,
    #[serde(rename = "24h")]
    H24,
    #[serde(other, rename = "auto")]
    Auto,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PanelMode {
    Several,
    Icon,
    #[serde(other)]
    Headline,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PanelIndicator {
    Bar,
    None,
    #[serde(other)]
    Ring,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PanelLabel {
    Window,
    None,
    #[serde(other)]
    Percent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum SpendPeriod {
    #[serde(rename = "today")]
    Today,
    #[serde(rename = "yesterday")]
    Yesterday,
    #[serde(rename = "7d")]
    Last7Days,
    #[serde(other, rename = "30d")]
    Last30Days,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpendUnit {
    Tokens,
    CostPerMtok,
    #[serde(other)]
    Cost,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpendBreakdown {
    Projects,
    #[serde(other)]
    Models,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize)]
pub struct PanelLimit {
    pub account_id: String,
    pub window: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(default)]
pub struct PanelPosition {
    #[serde(rename = "box")]
    pub panel_box: PanelBox,
    pub index: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PanelBox {
    Left,
    Center,
    #[default]
    #[serde(other)]
    Right,
}
