use headroom_core::pace::Tone;
use headroom_daemon::state::payload::{
    AccountStatus, AccountView, BalanceAmountView, BalanceView, StatePayload, WindowView,
};
use jiff::Timestamp;

use super::bar;
use super::format::{account_title, grouped, pace_note, percent_left, reset_text, usd};
use super::spend::spend_lines;
use super::style::Palette;
use super::table::pad;

const INDENT: &str = "  ";
const PERCENT_WIDTH: usize = 9;
const RESET_WIDTH: usize = "resets in 23h 59m".len();
const NO_ACCOUNTS: &str = "No AI coding tools found. Sign in with `codex` or `claude` first.";

pub fn render_status(state: &StatePayload, palette: Palette) -> String {
    let visible: Vec<&AccountView> = state.accounts.iter().filter(|a| !a.hidden).collect();
    let mut blocks: Vec<Vec<String>> = Vec::new();
    if visible.is_empty() {
        blocks.push(vec![palette.dim(NO_ACCOUNTS)]);
    }
    let layout = Layout::of(&visible);
    for account in &visible {
        blocks.push(account_block(account, &layout, state.generated_at, palette));
    }
    let spend = spend_lines(&state.usage, palette);
    if !spend.is_empty() {
        blocks.push(spend);
    }
    let hidden = state.accounts.len() - visible.len();
    if hidden > 0 {
        blocks.push(vec![palette.dim(&hidden_note(hidden))]);
    }
    let mut out = blocks
        .iter()
        .map(|lines| lines.join("\n"))
        .collect::<Vec<_>>()
        .join("\n\n");
    out.push('\n');
    out
}

fn hidden_note(count: usize) -> String {
    let noun = if count == 1 { "account" } else { "accounts" };
    format!("{count} hidden {noun} · headroom accounts show <ID>")
}

struct Layout {
    label: usize,
}

impl Layout {
    fn of(accounts: &[&AccountView]) -> Layout {
        let labels = accounts.iter().flat_map(|a| {
            let windows = a.windows.iter().map(|w| w.label.chars().count());
            windows.chain(a.balances.iter().map(|b| b.label.chars().count()))
        });
        Layout {
            label: labels.max().unwrap_or(0),
        }
    }
}

fn account_block(
    account: &AccountView,
    layout: &Layout,
    now: Timestamp,
    palette: Palette,
) -> Vec<String> {
    let mut lines = vec![header(account, palette)];
    if let Some(error) = &account.error {
        lines.push(format!(
            "{INDENT}{}",
            palette.tone(&error.message, error_tone(account))
        ));
    }
    lines.extend(
        account
            .notices
            .iter()
            .map(|notice| format!("{INDENT}{}", palette.tone(&notice.text, notice.tone))),
    );
    lines.extend(
        account
            .windows
            .iter()
            .map(|w| window_line(w, layout, now, palette)),
    );
    lines.extend(
        account
            .balances
            .iter()
            .map(|b| balance_line(b, layout, palette)),
    );
    if account.windows.is_empty() && account.balances.is_empty() && account.error.is_none() {
        lines.push(format!("{INDENT}{}", palette.dim("no data yet")));
    }
    lines
}

fn header(account: &AccountView, palette: Palette) -> String {
    let mut line = palette.bold(&account_title(account));
    if let Some(plan) = &account.plan {
        line.push_str("  ");
        line.push_str(&palette.dim(plan));
    }
    if let Some(tag) = status_tag(account.status, palette) {
        line.push_str("  ");
        line.push_str(&tag);
    }
    line
}

fn status_tag(status: AccountStatus, palette: Palette) -> Option<String> {
    match status {
        AccountStatus::Fresh => None,
        AccountStatus::Stale => Some(palette.dim("outdated")),
        AccountStatus::Refreshing => Some(palette.dim("refreshing…")),
        AccountStatus::Error => Some(palette.tone("couldn't refresh", Tone::Warning)),
        AccountStatus::SignedOut => Some(palette.tone("signed out", Tone::Critical)),
    }
}

fn error_tone(account: &AccountView) -> Tone {
    if account.status == AccountStatus::SignedOut {
        Tone::Critical
    } else {
        Tone::Warning
    }
}

fn window_line(window: &WindowView, layout: &Layout, now: Timestamp, palette: Palette) -> String {
    let percent = format!("{:>PERCENT_WIDTH$}", percent_left(window));
    let reset = reset_text(window.resets_at, now);
    let mut line = format!(
        "{INDENT}{}  {}  {}  {}",
        pad(&window.label, layout.label),
        bar::render(window, palette),
        palette.tone(&percent, window.tone),
        palette.dim(&reset),
    );
    if let Some(note) = pace_note(&window.pace, now) {
        line.push_str(&" ".repeat(RESET_WIDTH.saturating_sub(reset.chars().count()) + 2));
        line.push_str(&palette.tone(&note.text, note.tone));
    }
    line
}

fn balance_line(balance: &BalanceView, layout: &Layout, palette: Palette) -> String {
    let value = match &balance.amount {
        BalanceAmountView::Usd { usd_micros } => usd(*usd_micros),
        BalanceAmountView::Count { value, unit } => format!("{} {unit}", grouped(*value)),
    };
    format!(
        "{INDENT}{}  {}",
        pad(&balance.label, layout.label),
        palette.bold(&value)
    )
}

#[cfg(test)]
#[path = "status_tests.rs"]
mod tests;
