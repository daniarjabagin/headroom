use headroom_daemon::state::payload::{TotalsView, UsageView};

use super::format::{compact_tokens, usd};
use super::style::Palette;
use super::table::pad;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Spend {
    pub tokens: u64,
    pub cost_usd_micros: i64,
    pub partial: bool,
}

impl Spend {
    fn add(self, totals: &TotalsView) -> Spend {
        Spend {
            tokens: self.tokens.saturating_add(totals.tokens.total),
            cost_usd_micros: self.cost_usd_micros.saturating_add(totals.cost_usd_micros),
            partial: self.partial || totals.partial,
        }
    }

    pub fn summary(self) -> String {
        let partial = if self.partial { " (partial)" } else { "" };
        format!(
            "{} · {} tokens{partial}",
            usd(self.cost_usd_micros),
            compact_tokens(self.tokens)
        )
    }
}

pub fn total(usage: &[UsageView], period: Period) -> Spend {
    usage
        .iter()
        .fold(Spend::default(), |sum, view| sum.add(period(view)))
}

pub type Period = fn(&UsageView) -> &TotalsView;

pub const PERIODS: [(&str, Period); 3] = [
    ("Today", |view| &view.today),
    ("Yesterday", |view| &view.yesterday),
    ("30 days", |view| &view.last_30_days),
];

pub fn spend_lines(usage: &[UsageView], palette: Palette) -> Vec<String> {
    if usage.is_empty() {
        return Vec::new();
    }
    let rows: Vec<(&str, String, String, bool)> = PERIODS
        .iter()
        .map(|(name, period)| {
            let spend = total(usage, *period);
            let tokens = format!("{} tokens", compact_tokens(spend.tokens));
            (*name, usd(spend.cost_usd_micros), tokens, spend.partial)
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
