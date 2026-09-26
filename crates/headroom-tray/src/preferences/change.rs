use serde_json::{Value, json};

use super::model::{Headline, LogLevel, MAX_REFRESH_SECS, MIN_REFRESH_SECS, QuietHours};
use super::names::{
    density_name, language_name, log_level_name, panel_box_name, panel_indicator_name,
    panel_label_name, panel_mode_name, reset_format_name, spend_breakdown_name, spend_period_name,
    spend_unit_name, theme_name, time_format_name, value_mode_name,
};
use super::notify::{PROVIDER_THRESHOLD_RANGE, THRESHOLD_RANGE};
use crate::payload::{
    Density, Language, PanelIndicator, PanelLabel, PanelLimit, PanelMode, PanelPosition,
    ResetFormat, SpendBreakdown, SpendPeriod, SpendUnit, Theme, TimeFormat, ValueMode,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Section {
    Spend,
    AccountSpend,
    Trend,
    Forecast,
}

impl Section {
    pub const ALL: [Section; 4] = [
        Section::Spend,
        Section::AccountSpend,
        Section::Trend,
        Section::Forecast,
    ];

    fn field(self) -> &'static str {
        match self {
            Section::Spend => "show_spend",
            Section::AccountSpend => "show_account_spend",
            Section::Trend => "show_trend",
            Section::Forecast => "show_forecast",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Milestone {
    AlmostOut,
    CuttingItClose,
    WillRunOut,
    Reset,
}

impl Milestone {
    pub const ALL: [Milestone; 4] = [
        Milestone::AlmostOut,
        Milestone::CuttingItClose,
        Milestone::WillRunOut,
        Milestone::Reset,
    ];

    fn field(self) -> &'static str {
        match self {
            Milestone::AlmostOut => "almost_out",
            Milestone::CuttingItClose => "cutting_it_close",
            Milestone::WillRunOut => "will_run_out",
            Milestone::Reset => "reset",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Change {
    Theme(Theme),
    Language(Language),
    ValueMode(ValueMode),
    ResetFormat(ResetFormat),
    Section(Section, bool),
    CombineAccounts(bool),
    ShowBreakdown(bool),
    ReducedMotion(bool),
    RefreshInterval(u32),
    Headline(Headline),
    CheckUpdates(bool),
    Notify(Milestone, bool),
    HiddenWindows {
        account_id: String,
        windows: Vec<String>,
    },
    AdaptiveRefresh(bool),
    Density(Density),
    TimeFormat(TimeFormat),
    PanelMode(PanelMode),
    PanelIndicator(PanelIndicator),
    PanelLabel(PanelLabel),
    PanelLimits(Vec<PanelLimit>),
    PanelPosition(PanelPosition),
    SpendPeriod(SpendPeriod),
    SpendUnit(SpendUnit),
    SpendBreakdown(SpendBreakdown),
    StarredAccounts(Vec<String>),
    CollapseUnstarred(bool),
    HideOnScreenShare(bool),
    ThresholdPercent(u8),
    ProviderThreshold {
        provider: String,
        threshold: Option<u8>,
    },
    QuietHours(QuietHours),
    StatusPages(bool),
    Shortcut(String),
    LogLevel(LogLevel),
    OnboardingCompleted(bool),
}

fn display(field: &str, value: &Value) -> Value {
    json!({ "display": { field: value } })
}

fn headline_patch(headline: &Headline) -> Value {
    match headline {
        Headline::Auto => {
            json!({ "headline": { "mode": "auto", "account_id": null, "window": null } })
        }
        Headline::Pinned { account_id, window } => json!({
            "headline": { "mode": "pinned", "account_id": account_id, "window": window }
        }),
    }
}

fn hidden_windows_patch(account_id: &str, windows: &[String]) -> Value {
    let list = if windows.is_empty() {
        Value::Null
    } else {
        json!(windows)
    };
    json!({ "display": { "hidden_windows": { account_id: list } } })
}

fn deduped<T: Clone + PartialEq>(items: &[T]) -> Vec<T> {
    let mut unique: Vec<T> = Vec::with_capacity(items.len());
    for item in items {
        if !unique.contains(item) {
            unique.push(item.clone());
        }
    }
    unique
}

fn panel_limits_patch(limits: &[PanelLimit]) -> Value {
    let limits: Vec<Value> = deduped(limits)
        .iter()
        .map(|limit| json!({ "account_id": limit.account_id, "window": limit.window }))
        .collect();
    display("panel_limits", &json!(limits))
}

fn panel_position_patch(position: PanelPosition) -> Value {
    display(
        "panel_position",
        &json!({ "box": panel_box_name(position.panel_box), "index": position.index }),
    )
}

fn notifications(field: &str, value: &Value) -> Value {
    json!({ "notifications": { field: value } })
}

fn provider_threshold_patch(provider: &str, threshold: Option<u8>) -> Value {
    let value = threshold.map_or(Value::Null, |percent| {
        json!(percent.clamp(
            *PROVIDER_THRESHOLD_RANGE.start(),
            *PROVIDER_THRESHOLD_RANGE.end()
        ))
    });
    notifications("provider_thresholds", &json!({ provider: value }))
}

fn quiet_hours_patch(quiet: QuietHours) -> Value {
    notifications(
        "quiet_hours",
        &json!({
            "enabled": quiet.enabled,
            "from": quiet.from.to_string(),
            "to": quiet.to.to_string(),
            "allow_critical": quiet.allow_critical,
        }),
    )
}

fn named(field: &str, name: &str) -> Value {
    display(field, &json!(name))
}

fn threshold(percent: u8) -> Value {
    json!(percent.clamp(*THRESHOLD_RANGE.start(), *THRESHOLD_RANGE.end()))
}

impl Change {
    #[must_use]
    pub fn patch(&self) -> Value {
        match self {
            Change::Theme(theme) => named("theme", theme_name(*theme)),
            Change::Language(language) => named("language", language_name(*language)),
            Change::ValueMode(mode) => named("value_mode", value_mode_name(*mode)),
            Change::ResetFormat(format) => named("reset_format", reset_format_name(*format)),
            Change::Section(section, on) => display(section.field(), &json!(on)),
            Change::CombineAccounts(on) => display("combine_accounts", &json!(on)),
            Change::ShowBreakdown(on) => display("show_breakdown", &json!(on)),
            Change::ReducedMotion(on) => json!({ "reduced_motion": on }),
            Change::RefreshInterval(secs) => json!({
                "refresh_interval_secs": (*secs).clamp(MIN_REFRESH_SECS, MAX_REFRESH_SECS)
            }),
            Change::Headline(headline) => headline_patch(headline),
            Change::CheckUpdates(on) => json!({ "updates": { "check": on } }),
            Change::Notify(milestone, on) => notifications(milestone.field(), &json!(on)),
            Change::HiddenWindows {
                account_id,
                windows,
            } => hidden_windows_patch(account_id, windows),
            Change::AdaptiveRefresh(on) => json!({ "adaptive_refresh": on }),
            Change::Density(density) => named("density", density_name(*density)),
            Change::TimeFormat(format) => named("time_format", time_format_name(*format)),
            Change::PanelMode(mode) => named("panel_mode", panel_mode_name(*mode)),
            Change::PanelIndicator(indicator) => {
                named("panel_indicator", panel_indicator_name(*indicator))
            }
            Change::PanelLabel(label) => named("panel_label", panel_label_name(*label)),
            Change::PanelLimits(limits) => panel_limits_patch(limits),
            Change::PanelPosition(position) => panel_position_patch(*position),
            Change::SpendPeriod(period) => named("spend_period", spend_period_name(*period)),
            Change::SpendUnit(unit) => named("spend_unit", spend_unit_name(*unit)),
            Change::SpendBreakdown(breakdown) => {
                named("spend_breakdown", spend_breakdown_name(*breakdown))
            }
            Change::StarredAccounts(ids) => display("starred_accounts", &json!(deduped(ids))),
            Change::CollapseUnstarred(on) => display("collapse_unstarred", &json!(on)),
            Change::HideOnScreenShare(on) => display("hide_on_screen_share", &json!(on)),
            Change::ThresholdPercent(percent) => {
                notifications("threshold_percent", &threshold(*percent))
            }
            Change::ProviderThreshold {
                provider,
                threshold,
            } => provider_threshold_patch(provider, *threshold),
            Change::QuietHours(quiet) => quiet_hours_patch(*quiet),
            Change::StatusPages(on) => json!({ "status_pages": { "enabled": on } }),
            Change::Shortcut(accelerator) => json!({ "shortcuts": { "open": accelerator } }),
            Change::LogLevel(level) => json!({ "logging": { "level": log_level_name(*level) } }),
            Change::OnboardingCompleted(done) => json!({ "onboarding": { "completed": done } }),
        }
    }
}

#[cfg(test)]
#[path = "change_tests.rs"]
mod tests;
