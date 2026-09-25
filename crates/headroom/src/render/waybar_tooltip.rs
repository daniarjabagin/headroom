use headroom_core::pace::Tone;
use headroom_daemon::settings::ValueMode;
use headroom_daemon::state::payload::{AccountView, StatePayload, WindowView};
use jiff::Timestamp;

use super::format::{
    account_title, pace_note, percent_left, reset_text, rounded_percent, shown_windows,
};
use super::pango::{bold, escape, toned};
use super::spend::{PERIODS, summary};

const NO_DATA: &str = "No limits reported yet";

#[derive(Debug, Clone, Copy)]
pub struct TooltipStyle {
    pub rich: bool,
    pub mode: ValueMode,
}

impl TooltipStyle {
    pub const PLAIN: TooltipStyle = TooltipStyle {
        rich: false,
        mode: ValueMode::Left,
    };

    fn header(self, markup: &str) -> String {
        if self.rich {
            bold(markup)
        } else {
            markup.to_owned()
        }
    }

    fn toned(self, markup: &str, tone: Tone) -> String {
        if self.rich {
            toned(markup, tone)
        } else {
            markup.to_owned()
        }
    }
}

pub fn tooltip(state: &StatePayload, now: Timestamp, style: TooltipStyle) -> String {
    let mut lines: Vec<String> = Vec::new();
    for account in state.accounts.iter().filter(|a| !a.hidden) {
        lines.extend(account_lines(account, now, style));
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

fn account_lines(account: &AccountView, now: Timestamp, style: TooltipStyle) -> Vec<String> {
    let mut title = account_title(account);
    if let Some(plan) = &account.plan {
        title = format!("{title} ({plan})");
    }
    let mut lines = vec![style.header(&escape(&title))];
    if let Some(error) = &account.error {
        lines.push(format!("  {}", escape(&error.message)));
    }
    lines.extend(shown_windows(account).map(|window| window_line(window, now, style)));
    lines
}

fn window_line(window: &WindowView, now: Timestamp, style: TooltipStyle) -> String {
    let value = style.toned(&value_text(window, style.mode), window.tone);
    let mut parts = vec![value, reset_text(window.resets_at, now)];
    parts.extend(pace_note(&window.pace, now).map(|note| style.toned(&note.text, note.tone)));
    format!("  {}: {}", escape(&window.label), parts.join(" · "))
}

fn value_text(window: &WindowView, mode: ValueMode) -> String {
    match mode {
        ValueMode::Left => percent_left(window),
        ValueMode::Used => format!("{}% used", rounded_percent(window.used_percent)),
    }
}
