use super::model::LogLevel;
use crate::payload::{
    Density, Language, PanelBox, PanelIndicator, PanelLabel, PanelMode, ResetFormat,
    SpendBreakdown, SpendPeriod, SpendUnit, Theme, TimeFormat, ValueMode,
};

#[must_use]
pub fn theme_name(theme: Theme) -> &'static str {
    match theme {
        Theme::System => "system",
        Theme::Light => "light",
        Theme::Dark => "dark",
    }
}

#[must_use]
pub fn language_name(language: Language) -> &'static str {
    match language {
        Language::System => "system",
        Language::En => "en",
        Language::Ru => "ru",
    }
}

#[must_use]
pub fn value_mode_name(mode: ValueMode) -> &'static str {
    match mode {
        ValueMode::Left => "left",
        ValueMode::Used => "used",
    }
}

#[must_use]
pub fn reset_format_name(format: ResetFormat) -> &'static str {
    match format {
        ResetFormat::Countdown => "countdown",
        ResetFormat::Exact => "exact",
    }
}

#[must_use]
pub fn density_name(density: Density) -> &'static str {
    match density {
        Density::Normal => "normal",
        Density::Compact => "compact",
    }
}

#[must_use]
pub fn time_format_name(format: TimeFormat) -> &'static str {
    match format {
        TimeFormat::Auto => "auto",
        TimeFormat::H12 => "12h",
        TimeFormat::H24 => "24h",
    }
}

#[must_use]
pub fn panel_mode_name(mode: PanelMode) -> &'static str {
    match mode {
        PanelMode::Headline => "headline",
        PanelMode::Several => "several",
        PanelMode::Icon => "icon",
    }
}

#[must_use]
pub fn panel_indicator_name(indicator: PanelIndicator) -> &'static str {
    match indicator {
        PanelIndicator::Ring => "ring",
        PanelIndicator::Bar => "bar",
        PanelIndicator::None => "none",
    }
}

#[must_use]
pub fn panel_label_name(label: PanelLabel) -> &'static str {
    match label {
        PanelLabel::Percent => "percent",
        PanelLabel::Window => "window",
        PanelLabel::None => "none",
    }
}

#[must_use]
pub fn panel_box_name(panel_box: PanelBox) -> &'static str {
    match panel_box {
        PanelBox::Left => "left",
        PanelBox::Center => "center",
        PanelBox::Right => "right",
    }
}

#[must_use]
pub fn spend_period_name(period: SpendPeriod) -> &'static str {
    match period {
        SpendPeriod::Today => "today",
        SpendPeriod::Yesterday => "yesterday",
        SpendPeriod::Last7Days => "7d",
        SpendPeriod::Last30Days => "30d",
    }
}

#[must_use]
pub fn spend_unit_name(unit: SpendUnit) -> &'static str {
    match unit {
        SpendUnit::Cost => "cost",
        SpendUnit::Tokens => "tokens",
        SpendUnit::CostPerMtok => "cost_per_mtok",
    }
}

#[must_use]
pub fn spend_breakdown_name(breakdown: SpendBreakdown) -> &'static str {
    match breakdown {
        SpendBreakdown::Models => "models",
        SpendBreakdown::Projects => "projects",
    }
}

#[must_use]
pub fn log_level_name(level: LogLevel) -> &'static str {
    match level {
        LogLevel::Error => "error",
        LogLevel::Warn => "warn",
        LogLevel::Info => "info",
        LogLevel::Debug => "debug",
    }
}

#[cfg(test)]
mod tests {
    use serde::de::DeserializeOwned;

    use super::*;

    fn round_trips<T: DeserializeOwned + PartialEq + std::fmt::Debug + Copy>(
        values: &[T],
        name: fn(T) -> &'static str,
    ) {
        for value in values {
            let parsed: T = serde_json::from_value(serde_json::json!(name(*value))).unwrap();
            assert_eq!(parsed, *value);
        }
    }

    #[test]
    fn wire_names_parse_back() {
        round_trips(&[Density::Normal, Density::Compact], density_name);
        round_trips(
            &[TimeFormat::Auto, TimeFormat::H12, TimeFormat::H24],
            time_format_name,
        );
        round_trips(
            &[PanelMode::Headline, PanelMode::Several, PanelMode::Icon],
            panel_mode_name,
        );
        round_trips(
            &[
                PanelIndicator::Ring,
                PanelIndicator::Bar,
                PanelIndicator::None,
            ],
            panel_indicator_name,
        );
        round_trips(
            &[PanelLabel::Percent, PanelLabel::Window, PanelLabel::None],
            panel_label_name,
        );
        round_trips(
            &[PanelBox::Left, PanelBox::Center, PanelBox::Right],
            panel_box_name,
        );
        round_trips(
            &[
                SpendPeriod::Today,
                SpendPeriod::Yesterday,
                SpendPeriod::Last7Days,
                SpendPeriod::Last30Days,
            ],
            spend_period_name,
        );
        round_trips(
            &[SpendUnit::Cost, SpendUnit::Tokens, SpendUnit::CostPerMtok],
            spend_unit_name,
        );
        round_trips(
            &[SpendBreakdown::Models, SpendBreakdown::Projects],
            spend_breakdown_name,
        );
        round_trips(
            &[
                LogLevel::Error,
                LogLevel::Warn,
                LogLevel::Info,
                LogLevel::Debug,
            ],
            log_level_name,
        );
    }
}
