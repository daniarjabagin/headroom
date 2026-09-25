use headroom_core::pace::Tone;
use headroom_daemon::settings::ValueMode;
use headroom_daemon::state::payload::{StatePayload, WindowView};
use jiff::Timestamp;
use serde::Serialize;

use super::format::{rounded_percent, shown_windows};
use super::pango::{escape, toned};
use super::waybar_tooltip::{TooltipStyle, tooltip};
use crate::cli::{LabelStyle, WaybarArgs, WindowScope};

const DAEMON_ABSENT: &str = "Headroom daemon is not running";
const EMPTY_TEXT: &str = "—";
const VALUE_SEPARATOR: &str = " · ";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct WaybarLine {
    pub text: String,
    pub tooltip: String,
    pub class: &'static str,
    pub percentage: u8,
}

struct ProviderValue<'a> {
    name: String,
    window: Option<&'a WindowView>,
}

pub fn absent_line() -> WaybarLine {
    WaybarLine {
        text: EMPTY_TEXT.to_owned(),
        tooltip: DAEMON_ABSENT.to_owned(),
        class: tone_class(Tone::Neutral),
        percentage: 0,
    }
}

pub fn state_line(state: &StatePayload, now: Timestamp, args: &WaybarArgs) -> WaybarLine {
    if args.is_headline() {
        headline_line(state, now)
    } else {
        providers_line(state, now, args)
    }
}

fn headline_line(state: &StatePayload, now: Timestamp) -> WaybarLine {
    let tooltip = tooltip(state, now, TooltipStyle::PLAIN);
    match &state.headline {
        Some(headline) => WaybarLine {
            text: format!("{}%", rounded_percent(headline.remaining_percent)),
            tooltip,
            class: tone_class(headline.tone),
            percentage: gauge(headline.remaining_percent),
        },
        None => WaybarLine {
            text: EMPTY_TEXT.to_owned(),
            tooltip,
            class: tone_class(Tone::Neutral),
            percentage: 0,
        },
    }
}

fn providers_line(state: &StatePayload, now: Timestamp, args: &WaybarArgs) -> WaybarLine {
    let mode = state.display.value_mode;
    let scope = args.window.unwrap_or(WindowScope::Any);
    let values: Vec<ProviderValue> = provider_ids(state, args)
        .iter()
        .map(|id| provider_value(state, id, scope))
        .collect();
    let windows: Vec<&WindowView> = values.iter().filter_map(|value| value.window).collect();
    let labels = args.labels.unwrap_or(LabelStyle::Full);
    let text: Vec<String> = values.iter().map(|v| value_text(v, labels, mode)).collect();
    WaybarLine {
        text: if text.is_empty() {
            EMPTY_TEXT.to_owned()
        } else {
            text.join(VALUE_SEPARATOR)
        },
        tooltip: tooltip(state, now, TooltipStyle { rich: true, mode }),
        class: tone_class(windows.iter().map(|w| w.tone).max().unwrap_or_default()),
        percentage: windows
            .iter()
            .map(|w| w.remaining_percent)
            .min_by(f64::total_cmp)
            .map_or(0, gauge),
    }
}

fn provider_ids(state: &StatePayload, args: &WaybarArgs) -> Vec<String> {
    if !args.providers.is_empty() {
        return args.providers.clone();
    }
    let mut ids: Vec<String> = Vec::new();
    for account in state.accounts.iter().filter(|account| !account.hidden) {
        let id = account.provider.as_str();
        if !ids.iter().any(|seen| seen == id) {
            ids.push(id.to_owned());
        }
    }
    ids
}

fn provider_value<'a>(state: &'a StatePayload, id: &str, scope: WindowScope) -> ProviderValue<'a> {
    let accounts: Vec<_> = state
        .accounts
        .iter()
        .filter(|account| !account.hidden && account.provider.as_str() == id)
        .collect();
    let window = accounts
        .iter()
        .flat_map(|account| shown_windows(account))
        .filter(|window| scope.includes(&window.id))
        .min_by(|a, b| a.remaining_percent.total_cmp(&b.remaining_percent));
    let name = accounts
        .first()
        .map_or_else(|| id.to_owned(), |account| account.provider_name.clone());
    ProviderValue { name, window }
}

fn value_text(value: &ProviderValue, labels: LabelStyle, mode: ValueMode) -> String {
    let number = match value.window {
        Some(window) => toned(&format!("{}%", shown_percent(window, mode)), window.tone),
        None => EMPTY_TEXT.to_owned(),
    };
    match labels {
        LabelStyle::Full => format!("{} {number}", escape(&value.name)),
        LabelStyle::None => number,
    }
}

fn shown_percent(window: &WindowView, mode: ValueMode) -> String {
    match mode {
        ValueMode::Left => rounded_percent(window.remaining_percent),
        ValueMode::Used => rounded_percent(window.used_percent),
    }
}

fn tone_class(tone: Tone) -> &'static str {
    match tone {
        Tone::Good => "good",
        Tone::Warning => "warning",
        Tone::Critical => "critical",
        Tone::Neutral => "neutral",
    }
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the value is rounded and clamped to 0..=100 first"
)]
fn gauge(remaining_percent: f64) -> u8 {
    remaining_percent.clamp(0.0, 100.0).round() as u8
}

#[cfg(test)]
#[path = "waybar_tests.rs"]
mod tests;
