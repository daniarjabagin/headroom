use super::capability::Capabilities;
use super::choices::Choice;
use super::model::{ClockTime, LogLevel};
use crate::dates::{Clock, hour_minute};
use crate::i18n::{Lang, fill};
use crate::payload::{
    Density, PanelIndicator, PanelLabel, PanelMode, Spend, SpendBreakdown, SpendPeriod, SpendUnit,
    TimeFormat,
};

const THRESHOLD_PRESETS: [u8; 4] = [5, 10, 20, 30];
const QUIET_STEP_MINUTES: u8 = 30;

fn labeled<T: Copy>(lang: Lang, items: &[(T, &'static str)]) -> Vec<Choice<T>> {
    items
        .iter()
        .map(|(value, label)| Choice {
            value: *value,
            label: lang.tr(label).to_owned(),
        })
        .collect()
}

#[must_use]
pub fn density_choices(lang: Lang) -> Vec<Choice<Density>> {
    labeled(
        lang,
        &[(Density::Normal, "Normal"), (Density::Compact, "Compact")],
    )
}

#[must_use]
pub fn time_format_choices(lang: Lang) -> Vec<Choice<TimeFormat>> {
    labeled(
        lang,
        &[
            (TimeFormat::Auto, "Automatic"),
            (TimeFormat::H24, "24-hour"),
            (TimeFormat::H12, "12-hour"),
        ],
    )
}

#[must_use]
pub fn panel_mode_choices(lang: Lang) -> Vec<Choice<PanelMode>> {
    labeled(
        lang,
        &[
            (PanelMode::Headline, "One limit"),
            (PanelMode::Several, "Several limits"),
            (PanelMode::Icon, "Icon only"),
        ],
    )
}

#[must_use]
pub fn panel_indicator_choices(lang: Lang) -> Vec<Choice<PanelIndicator>> {
    labeled(
        lang,
        &[
            (PanelIndicator::Ring, "Ring"),
            (PanelIndicator::Bar, "Bar"),
            (PanelIndicator::None, "None"),
        ],
    )
}

#[must_use]
pub fn panel_label_choices(lang: Lang, capabilities: Capabilities) -> Vec<Choice<PanelLabel>> {
    let mut choices = labeled(
        lang,
        &[
            (PanelLabel::Percent, "Percent"),
            (PanelLabel::Window, "Limit name"),
        ],
    );
    if capabilities.release_0_6 {
        choices.extend(labeled(lang, &[(PanelLabel::None, "Nothing")]));
    }
    choices
}

#[must_use]
pub fn spend_period_choices(lang: Lang, spend: Option<&Spend>) -> Vec<Choice<SpendPeriod>> {
    let all = labeled(
        lang,
        &[
            (SpendPeriod::Today, "Today"),
            (SpendPeriod::Yesterday, "Yesterday"),
            (SpendPeriod::Last7Days, "7 Days"),
            (SpendPeriod::Last30Days, "30 Days"),
        ],
    );
    all.into_iter()
        .filter(|choice| spend.is_none_or(|spend| spend.has_period(choice.value)))
        .collect()
}

#[must_use]
pub fn spend_unit_choices(lang: Lang) -> Vec<Choice<SpendUnit>> {
    labeled(
        lang,
        &[
            (SpendUnit::Cost, "Cost"),
            (SpendUnit::Tokens, "Tokens"),
            (SpendUnit::CostPerMtok, "Cost per 1M tokens"),
        ],
    )
}

#[must_use]
pub fn spend_breakdown_choices(lang: Lang, spend: Option<&Spend>) -> Vec<Choice<SpendBreakdown>> {
    let mut choices = labeled(lang, &[(SpendBreakdown::Models, "Models")]);
    if spend.is_none_or(Spend::has_projects) {
        choices.extend(labeled(lang, &[(SpendBreakdown::Projects, "Projects")]));
    }
    choices
}

#[must_use]
pub fn log_level_choices(lang: Lang) -> Vec<Choice<LogLevel>> {
    labeled(
        lang,
        &[
            (LogLevel::Error, "Errors only"),
            (LogLevel::Warn, "Warnings"),
            (LogLevel::Info, "Info"),
            (LogLevel::Debug, "Debug"),
        ],
    )
}

#[must_use]
pub fn threshold_label(lang: Lang, percent: u8) -> String {
    fill(
        lang.tr("Under {percent}% left"),
        &[("percent", &percent.to_string())],
    )
}

fn with_current(presets: &[u8], current: Option<u8>) -> Vec<u8> {
    let mut values = presets.to_vec();
    if let Some(current) = current.filter(|value| !values.contains(value)) {
        values.push(current);
        values.sort_unstable();
    }
    values
}

#[must_use]
pub fn threshold_choices(lang: Lang, current: u8) -> Vec<Choice<u8>> {
    with_current(&THRESHOLD_PRESETS, Some(current))
        .into_iter()
        .map(|value| Choice {
            value,
            label: threshold_label(lang, value),
        })
        .collect()
}

#[must_use]
pub fn provider_threshold_choices(
    lang: Lang,
    general: u8,
    current: Option<u8>,
) -> Vec<Choice<Option<u8>>> {
    let default_label = fill(
        lang.tr("Same as all ({percent}%)"),
        &[("percent", &general.to_string())],
    );
    let mut choices = vec![
        Choice {
            value: None,
            label: default_label,
        },
        Choice {
            value: Some(0),
            label: lang.tr("Off").to_owned(),
        },
    ];
    let current = current.filter(|value| *value != 0);
    choices.extend(
        with_current(&THRESHOLD_PRESETS, current)
            .into_iter()
            .map(|value| Choice {
                value: Some(value),
                label: threshold_label(lang, value),
            }),
    );
    choices
}

fn clock_label(time: ClockTime, clock: Clock) -> String {
    hour_minute(
        time.hour().cast_signed(),
        time.minute().cast_signed(),
        clock,
    )
}

#[must_use]
pub fn quiet_time_choices(clock: Clock, current: ClockTime) -> Vec<Choice<ClockTime>> {
    let mut times: Vec<ClockTime> = (0..24u8)
        .flat_map(|hour| {
            (0..60u8)
                .step_by(usize::from(QUIET_STEP_MINUTES))
                .filter_map(move |minute| ClockTime::new(hour, minute))
        })
        .collect();
    if !times.contains(&current) {
        times.push(current);
        times.sort_unstable();
    }
    times
        .into_iter()
        .map(|time| Choice {
            value: time,
            label: clock_label(time, clock),
        })
        .collect()
}

#[cfg(test)]
#[path = "options_tests.rs"]
mod tests;
