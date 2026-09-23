use headroom_daemon::state::payload::{PeriodSpendView, SpendView};

use super::format::{compact_tokens, usd};
use super::style::Palette;
use super::table::pad;

pub type Period = fn(&SpendView) -> &PeriodSpendView;

pub const PERIODS: [(&str, Period); 3] = [
    ("Today", |spend| &spend.today),
    ("Yesterday", |spend| &spend.yesterday),
    ("30 days", |spend| &spend.last_30_days),
];

pub fn summary(period: &PeriodSpendView) -> String {
    let partial = if period.partial { " (partial)" } else { "" };
    format!(
        "{} · {} tokens{partial}",
        usd(period.cost_usd_micros),
        compact_tokens(period.total_tokens)
    )
}

pub fn spend_lines(spend: &SpendView, palette: Palette) -> Vec<String> {
    let rows: Vec<(&str, String, String, bool)> = PERIODS
        .iter()
        .map(|(name, period)| {
            let totals = period(spend);
            let tokens = format!("{} tokens", compact_tokens(totals.total_tokens));
            (*name, usd(totals.cost_usd_micros), tokens, totals.partial)
        })
        .collect();
    let cost_width = rows.iter().map(|row| row.1.len()).max().unwrap_or(0);
    let token_width = rows.iter().map(|row| row.2.len()).max().unwrap_or(0);
    let mut lines = vec![format!(
        "{}  {}",
        palette.bold("Spend"),
        palette.dim("estimated from local logs")
    )];
    lines.extend(rows.iter().map(|(name, cost, tokens, partial)| {
        let mut line = format!(
            "  {}  {}  {}",
            pad(name, "Yesterday".len()),
            palette.bold(&format!("{cost:>cost_width$}")),
            palette.dim(&format!("{tokens:>token_width$}")),
        );
        if *partial {
            line.push_str("  ");
            line.push_str(&palette.dim("partial"));
        }
        line
    }));
    lines
}
