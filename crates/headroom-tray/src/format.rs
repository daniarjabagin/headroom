use jiff::Timestamp;

use crate::dates::{Locale, exact_moment};
use crate::i18n::{Lang, fill};
use crate::labels::label_text;
use crate::numbers::{compact_tokens, exact_usd, usd};
use crate::payload::{
    Display, ModelUsage, PeriodSpend, ProviderSpend, ResetFormat, Severity, SpendUnit, ValueMode,
    Window,
};

const SECOND: i64 = 1000;
const MINUTE: i64 = 60 * SECOND;
const HOUR: i64 = 60 * MINUTE;
const DAY: i64 = 24 * HOUR;
const ELLIPSIS: char = '\u{2026}';

#[must_use]
pub fn millis_between(from: Timestamp, to: Timestamp) -> i64 {
    to.as_millisecond() - from.as_millisecond()
}

#[must_use]
#[allow(
    clippy::cast_possible_truncation,
    reason = "percentages are bounded display values; rounding happens only here, at the edge"
)]
pub fn round_percent(value: f64) -> i64 {
    value.round().max(0.0) as i64
}

fn percent_template(lang: Lang, template: &'static str, percent: f64) -> String {
    fill(
        lang.tr(template),
        &[("percent", &round_percent(percent).to_string())],
    )
}

#[must_use]
pub fn reading_percent(window: &Window, mode: ValueMode) -> f64 {
    match mode {
        ValueMode::Left => window.remaining_percent,
        ValueMode::Used => window.used_percent,
    }
}

#[must_use]
pub fn percent_reading(lang: Lang, percent: f64, mode: ValueMode) -> String {
    match mode {
        ValueMode::Left => percent_template(lang, "{percent}% left", percent),
        ValueMode::Used => percent_template(lang, "{percent}% used", percent),
    }
}

#[must_use]
pub fn window_label(lang: Lang, id: &str, label: &str) -> String {
    match id {
        "session" => lang.tr("Session").to_owned(),
        "weekly" => lang.tr("Weekly").to_owned(),
        _ if label.is_empty() => id.to_owned(),
        _ => label_text(lang, label),
    }
}

fn short_duration(lang: Lang, ms: i64) -> String {
    let minutes = ms / MINUTE;
    let seconds = (ms % MINUTE) / SECOND;
    if minutes == 0 {
        return fill(
            lang.tr("{seconds}s"),
            &[("seconds", &seconds.max(1).to_string())],
        );
    }
    fill(
        lang.tr("{minutes}m {seconds}s"),
        &[
            ("minutes", &minutes.to_string()),
            ("seconds", &format!("{seconds:02}")),
        ],
    )
}

#[must_use]
pub fn duration(lang: Lang, ms: i64, with_seconds: bool) -> String {
    if with_seconds && ms < HOUR {
        return short_duration(lang, ms);
    }
    let days = ms / DAY;
    let hours = (ms % DAY) / HOUR;
    let minutes = (ms % HOUR) / MINUTE;
    if days > 0 {
        let values = [("days", days.to_string()), ("hours", hours.to_string())];
        return fill_owned(lang.tr("{days}d {hours}h"), &values);
    }
    if hours > 0 {
        let values = [
            ("hours", hours.to_string()),
            ("minutes", minutes.to_string()),
        ];
        return fill_owned(lang.tr("{hours}h {minutes}m"), &values);
    }
    fill(
        lang.tr("{minutes}m"),
        &[("minutes", &minutes.max(1).to_string())],
    )
}

fn fill_owned(template: &str, values: &[(&str, String)]) -> String {
    let borrowed: Vec<(&str, &str)> = values.iter().map(|(k, v)| (*k, v.as_str())).collect();
    fill(template, &borrowed)
}

#[must_use]
pub fn is_countdown_live(resets_at: Option<Timestamp>, now: Timestamp) -> bool {
    resets_at.is_some_and(|reset| {
        let left = millis_between(now, reset);
        left > 0 && left < HOUR
    })
}

#[must_use]
pub fn reset_phrase(
    locale: &Locale,
    resets_at: Timestamp,
    now: Timestamp,
    format: ResetFormat,
    with_seconds: bool,
) -> String {
    let lang = locale.lang;
    let left = millis_between(now, resets_at);
    if left <= 0 {
        return lang.tr("reset pending").to_owned();
    }
    if format == ResetFormat::Exact {
        let moment = exact_moment(resets_at, now, locale);
        return fill(lang.tr("resets {moment}"), &[("moment", &moment)]);
    }
    if left < if with_seconds { SECOND } else { MINUTE } {
        return lang.tr("resets soon").to_owned();
    }
    let duration = duration(lang, left, with_seconds);
    fill(lang.tr("resets in {duration}"), &[("duration", &duration)])
}

fn capitalized(text: &str) -> String {
    let mut chars = text.chars();
    chars.next().map_or_else(String::new, |first| {
        first.to_uppercase().chain(chars).collect()
    })
}

#[must_use]
pub fn reset_text(
    locale: &Locale,
    resets_at: Option<Timestamp>,
    now: Timestamp,
    format: ResetFormat,
) -> String {
    match resets_at {
        None => locale.lang.tr("Not started").to_owned(),
        Some(reset) => capitalized(&reset_phrase(locale, reset, now, format, true)),
    }
}

#[must_use]
pub fn spare_text(lang: Lang, spare_percent: f64) -> String {
    percent_template(lang, "~{percent}% spare", spare_percent)
}

#[must_use]
pub fn limit_text(lang: Lang, runs_out_at: Option<Timestamp>, now: Timestamp) -> String {
    match runs_out_at {
        Some(at) if at > now => {
            let left = duration(lang, millis_between(now, at), false);
            fill(lang.tr("Limit in {duration}"), &[("duration", &left)])
        }
        _ => lang.tr("Limit soon").to_owned(),
    }
}

fn run_out_forecast(
    locale: &Locale,
    window: &Window,
    now: Timestamp,
    format: ResetFormat,
) -> String {
    let lang = locale.lang;
    let Some(runs_out_at) = window.pace.runs_out_at.filter(|at| *at > now) else {
        return lang.tr("At this pace: runs out any minute").to_owned();
    };
    let left = duration(lang, millis_between(now, runs_out_at), false);
    let runs_out = fill(lang.tr("runs out in {duration}"), &[("duration", &left)]);
    match window.resets_at {
        None => fill(
            lang.tr("At this pace: {runsOut}"),
            &[("runsOut", &runs_out)],
        ),
        Some(reset) => {
            let resets = reset_phrase(locale, reset, now, format, false);
            fill(
                lang.tr("At this pace: {runsOut} · {resets}"),
                &[("runsOut", &runs_out), ("resets", &resets)],
            )
        }
    }
}

fn at_reset_forecast(lang: Lang, window: &Window, spare: f64, mode: ValueMode) -> String {
    match (mode, window.pace.projected_percent) {
        (ValueMode::Used, Some(projected)) => {
            percent_template(lang, "At this pace: ~{percent}% used at reset", projected)
        }
        _ => percent_template(lang, "At this pace: ~{percent}% left at reset", spare),
    }
}

#[must_use]
pub fn forecast_text(
    locale: &Locale,
    window: &Window,
    now: Timestamp,
    display: &Display,
) -> Option<String> {
    match (window.pace.severity, window.pace.spare_percent) {
        (Severity::RunningOut, _) => {
            Some(run_out_forecast(locale, window, now, display.reset_format))
        }
        (Severity::Healthy | Severity::Close, Some(spare)) => Some(at_reset_forecast(
            locale.lang,
            window,
            spare,
            display.value_mode,
        )),
        _ => None,
    }
}

#[must_use]
pub fn next_update_text(lang: Lang, next_refresh_at: Timestamp, now: Timestamp) -> String {
    let left = millis_between(now, next_refresh_at);
    if left < MINUTE {
        return lang.tr("Next update in <1m").to_owned();
    }
    let text = duration(lang, left, false);
    fill(lang.tr("Next update in {duration}"), &[("duration", &text)])
}

#[must_use]
pub fn ago_text(lang: Lang, moment: Timestamp, now: Timestamp) -> String {
    let elapsed = millis_between(moment, now);
    if elapsed < MINUTE {
        return lang.tr("just now").to_owned();
    }
    let text = duration(lang, elapsed, false);
    fill(lang.tr("{duration} ago"), &[("duration", &text)])
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpendFigures {
    pub cost_usd_micros: i64,
    pub total_tokens: u64,
    pub cost_per_mtok_usd_micros: Option<i64>,
}

impl From<&PeriodSpend> for SpendFigures {
    fn from(period: &PeriodSpend) -> Self {
        Self {
            cost_usd_micros: period.cost_usd_micros,
            total_tokens: period.total_tokens,
            cost_per_mtok_usd_micros: period.cost_per_mtok_usd_micros,
        }
    }
}

impl From<&ProviderSpend> for SpendFigures {
    fn from(provider: &ProviderSpend) -> Self {
        Self {
            cost_usd_micros: provider.cost_usd_micros,
            total_tokens: provider.total_tokens,
            cost_per_mtok_usd_micros: provider.cost_per_mtok_usd_micros,
        }
    }
}

impl From<&ModelUsage> for SpendFigures {
    fn from(model: &ModelUsage) -> Self {
        Self {
            cost_usd_micros: model.cost_usd_micros,
            total_tokens: model.total_tokens,
            cost_per_mtok_usd_micros: model.cost_per_mtok_usd_micros,
        }
    }
}

#[must_use]
pub fn cost_per_mtok_text(lang: Lang, micros: Option<i64>) -> String {
    match micros {
        Some(value) => fill(
            lang.tr("{cost} / 1M tokens"),
            &[("cost", &exact_usd(value))],
        ),
        None => lang.tr("unpriced").to_owned(),
    }
}

#[must_use]
pub fn spend_value_text(lang: Lang, unit: SpendUnit, figures: SpendFigures) -> String {
    match unit {
        SpendUnit::Cost => usd(figures.cost_usd_micros),
        SpendUnit::Tokens => compact_tokens(lang, figures.total_tokens),
        SpendUnit::CostPerMtok => cost_per_mtok_text(lang, figures.cost_per_mtok_usd_micros),
    }
}

#[must_use]
pub fn project_label(lang: Lang, project: Option<&str>) -> String {
    project.map_or_else(|| lang.tr("No project").to_owned(), str::to_owned)
}

#[must_use]
pub fn other_projects_label(lang: Lang, count: u64) -> String {
    let forms = ["{count} other project", "{count} other projects"];
    fill(
        lang.tr_plural(forms, count),
        &[("count", &count.to_string())],
    )
}

fn last_segment_chars(text: &str) -> usize {
    text.rsplit('/')
        .next()
        .map_or(0, |segment| segment.chars().count())
}

#[must_use]
pub fn middle_ellipsis(text: &str, max_chars: usize) -> String {
    let count = text.chars().count();
    if count <= max_chars || max_chars < 3 {
        return text.to_owned();
    }
    let room = max_chars - 1;
    let tail = (last_segment_chars(text) + 1).min(room.saturating_sub(4).max(1));
    let head: String = text.chars().take(room - tail).collect();
    let end: String = text.chars().skip(count - tail).collect();
    format!("{head}{ELLIPSIS}{end}")
}

#[cfg(test)]
#[path = "format_tests.rs"]
mod tests;
