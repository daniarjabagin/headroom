use jiff::Timestamp;
use serde::Deserialize;

use crate::dates::Locale;
use crate::format::{forecast_text, reset_text, round_percent, window_label};
use crate::i18n::{Lang, fill};
use crate::payload::{Account, Display, Pace, Tone, ValueMode, Window};
use crate::quota::{PaceNote, pace_note, tick_position};

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct CombinedGroup {
    pub provider: String,
    pub provider_name: String,
    pub account_ids: Vec<String>,
    pub accounts: Vec<CombinedAccount>,
    pub windows: Vec<CombinedWindow>,
    #[serde(default)]
    pub collapsed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct CombinedAccount {
    pub account_id: String,
    pub label: Option<String>,
    pub plan: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct CombinedWindow {
    pub id: String,
    pub label: String,
    pub capacity_percent: f64,
    pub remaining_percent: f64,
    pub used_percent: f64,
    pub resets_at: Option<Timestamp>,
    pub tone: Tone,
    pub pace: Pace,
    pub segments: Vec<Segment>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Segment {
    pub account_id: String,
    pub label: Option<String>,
    pub remaining_percent: f64,
    pub used_percent: f64,
    pub resets_at: Option<Timestamp>,
    pub tone: Tone,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SegmentFill {
    pub fraction: f64,
    pub tone: Tone,
    pub tick: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CombinedRow {
    pub label: String,
    pub headline: String,
    pub note: Option<PaceNote>,
    pub trailing: String,
    pub forecast: Option<String>,
    pub segments: Vec<SegmentFill>,
    pub breakdown: String,
}

#[must_use]
pub fn group_title(lang: Lang, group: &CombinedGroup) -> String {
    let count = u64::try_from(group.accounts.len()).unwrap_or(u64::MAX);
    let accounts = fill(
        lang.tr_plural(["{count} account", "{count} accounts"], count),
        &[("count", &count.to_string())],
    );
    format!("{} · {accounts}", group.provider_name)
}

#[must_use]
pub fn group_plans(group: &CombinedGroup) -> Option<String> {
    let plans: Vec<&str> = group
        .accounts
        .iter()
        .filter_map(|account| account.plan.as_deref())
        .collect();
    (!plans.is_empty()).then(|| plans.join(" · "))
}

fn reading(lang: Lang, percent: f64, capacity: f64, mode: ValueMode) -> String {
    let template = match mode {
        ValueMode::Left => "{percent}% left of {capacity}%",
        ValueMode::Used => "{percent}% used of {capacity}%",
    };
    fill(
        lang.tr(template),
        &[
            ("percent", &round_percent(percent).to_string()),
            ("capacity", &round_percent(capacity).to_string()),
        ],
    )
}

fn as_window(window: &CombinedWindow) -> Window {
    Window {
        id: window.id.clone(),
        label: window.label.clone(),
        used_percent: window.used_percent,
        remaining_percent: window.remaining_percent,
        resets_at: window.resets_at,
        tone: window.tone,
        pace: window.pace.clone(),
        hidden: false,
    }
}

fn segment_tick(
    members: &[Account],
    segment: &Segment,
    window_id: &str,
    display: &Display,
) -> Option<f64> {
    members
        .iter()
        .find(|account| account.id == segment.account_id)?
        .windows
        .iter()
        .find(|window| window.id == window_id)
        .and_then(|window| tick_position(window, display))
}

fn segment_fill(
    members: &[Account],
    segment: &Segment,
    window_id: &str,
    display: &Display,
) -> SegmentFill {
    SegmentFill {
        fraction: (segment_percent(segment, display.value_mode) / 100.0).clamp(0.0, 1.0),
        tone: segment.tone,
        tick: segment_tick(members, segment, window_id, display),
    }
}

fn segment_name(segment: &Segment) -> &str {
    segment.label.as_deref().unwrap_or(&segment.account_id)
}

fn segment_percent(segment: &Segment, mode: ValueMode) -> f64 {
    match mode {
        ValueMode::Left => segment.remaining_percent,
        ValueMode::Used => segment.used_percent,
    }
}

fn breakdown(
    locale: &Locale,
    window: &CombinedWindow,
    display: &Display,
    now: Timestamp,
) -> String {
    let title = window
        .segments
        .iter()
        .map(|segment| {
            let percent = round_percent(segment_percent(segment, display.value_mode));
            format!("{} {percent}%", segment_name(segment))
        })
        .collect::<Vec<_>>()
        .join(" · ");
    let resets = window.segments.iter().map(|segment| {
        let reset = reset_text(locale, segment.resets_at, now, display.reset_format);
        format!("{} · {reset}", segment_name(segment))
    });
    std::iter::once(title)
        .chain(resets)
        .collect::<Vec<_>>()
        .join("\n")
}

#[must_use]
pub fn combined_row(
    locale: &Locale,
    window: &CombinedWindow,
    members: &[Account],
    display: &Display,
    now: Timestamp,
) -> CombinedRow {
    let percent = match display.value_mode {
        ValueMode::Left => window.remaining_percent,
        ValueMode::Used => window.used_percent,
    };
    let plain = as_window(window);
    CombinedRow {
        label: window_label(locale.lang, &window.id, &window.label),
        headline: reading(
            locale.lang,
            percent,
            window.capacity_percent,
            display.value_mode,
        ),
        note: pace_note(locale, &plain, now, display.show_forecast),
        trailing: reset_text(locale, window.resets_at, now, display.reset_format),
        forecast: display
            .show_forecast
            .then(|| forecast_text(locale, &plain, now, display))
            .flatten(),
        segments: window
            .segments
            .iter()
            .map(|segment| segment_fill(members, segment, &window.id, display))
            .collect(),
        breakdown: breakdown(locale, window, display, now),
    }
}

#[cfg(test)]
#[path = "combined_tests.rs"]
mod tests;
