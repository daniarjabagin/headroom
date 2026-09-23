use headroom_core::pace::Tone;
use headroom_daemon::state::payload::{AccountView, Headline, StatePayload, WindowView};
use jiff::Timestamp;
use serde::Serialize;

use super::format::{account_title, pace_note, percent_left, reset_text, rounded_percent};
use super::spend::{PERIODS, summary};

const DAEMON_ABSENT: &str = "Headroom daemon is not running";
const NO_DATA: &str = "No limits reported yet";
const EMPTY_TEXT: &str = "—";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct WaybarLine {
    pub text: String,
    pub tooltip: String,
    pub class: &'static str,
    pub percentage: u8,
}

pub fn absent_line() -> WaybarLine {
    WaybarLine {
        text: EMPTY_TEXT.to_owned(),
        tooltip: DAEMON_ABSENT.to_owned(),
        class: tone_class(Tone::Neutral),
        percentage: 0,
    }
}

pub fn state_line(state: &StatePayload, now: Timestamp) -> WaybarLine {
    let tooltip = escape_markup(&tooltip(state, now));
    match &state.headline {
        Some(headline) => WaybarLine {
            text: format!("{}%", rounded_percent(headline.remaining_percent)),
            tooltip,
            class: tone_class(headline.tone),
            percentage: gauge(headline),
        },
        None => WaybarLine {
            text: EMPTY_TEXT.to_owned(),
            tooltip,
            class: tone_class(Tone::Neutral),
            percentage: 0,
        },
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
fn gauge(headline: &Headline) -> u8 {
    headline.remaining_percent.clamp(0.0, 100.0).round() as u8
}

fn tooltip(state: &StatePayload, now: Timestamp) -> String {
    let mut lines: Vec<String> = Vec::new();
    for account in state.accounts.iter().filter(|a| !a.hidden) {
        lines.extend(account_lines(account, now));
    }
    if lines.is_empty() {
        lines.push(NO_DATA.to_owned());
    }
    if !state.usage.is_empty() {
        lines.push(String::new());
        lines.extend(
            PERIODS
                .iter()
                .map(|(name, period)| format!("{name}: {}", summary(period(&state.spend)))),
        );
    }
    lines.join("\n")
}

fn account_lines(account: &AccountView, now: Timestamp) -> Vec<String> {
    let mut title = account_title(account);
    if let Some(plan) = &account.plan {
        title = format!("{title} ({plan})");
    }
    let mut lines = vec![title];
    if let Some(error) = &account.error {
        lines.push(format!("  {}", error.message));
    }
    lines.extend(
        account
            .windows
            .iter()
            .map(|window| window_line(window, now)),
    );
    lines
}

fn window_line(window: &WindowView, now: Timestamp) -> String {
    let mut parts = vec![percent_left(window), reset_text(window.resets_at, now)];
    parts.extend(pace_note(&window.pace, now).map(|note| note.text));
    format!("  {}: {}", window.label, parts.join(" · "))
}

fn escape_markup(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[cfg(test)]
#[path = "waybar_tests.rs"]
mod tests;
