use std::collections::BTreeMap;

use headroom_core::usage::{ModelUsage, UsageSummary, UsageTotals};
use jiff::ToSpan;
use jiff::civil::Date;

use super::AssembleContext;
use super::payload::{DailyView, ModelView, TokensView, TotalsView, UsageView};
use crate::home::UsageHome;

const TREND_DAYS: usize = 30;

#[must_use]
pub fn usage_view(
    home: &UsageHome,
    summary: &UsageSummary,
    ctx: &AssembleContext<'_>,
) -> UsageView {
    let today = ctx.tz.to_datetime(ctx.now).date();
    UsageView {
        provider: home.provider,
        usage_home: ctx.homes.show(&home.home),
        today: totals_view(&summary.today),
        yesterday: totals_view(&summary.yesterday),
        last_30_days: totals_view(&summary.last_30_days),
        daily: daily(summary, today),
        models: summary.models.iter().map(model_view).collect(),
    }
}

fn totals_view(totals: &UsageTotals) -> TotalsView {
    let tokens = &totals.tokens;
    TotalsView {
        tokens: TokensView {
            input: tokens.input.0,
            cache_read: tokens.cache_read.0,
            cache_write: tokens.cache_write().0,
            output: tokens.output.0,
            reasoning: tokens.reasoning.0,
            total: tokens.total().0,
        },
        cost_usd_micros: totals.cost.0,
        partial: totals.is_partial(),
        unpriced_tokens: totals.unpriced_tokens.0,
        unpriced_models: totals.unpriced_models.iter().cloned().collect(),
    }
}

fn daily(summary: &UsageSummary, today: Date) -> Vec<DailyView> {
    let by_date: BTreeMap<Date, &UsageTotals> =
        summary.daily.iter().map(|(d, t)| (*d, t)).collect();
    let Ok(first) = today.checked_sub(29.days()) else {
        return by_date
            .into_iter()
            .map(|(d, t)| daily_view(d, Some(t)))
            .collect();
    };
    first
        .series(1.day())
        .take(TREND_DAYS)
        .map(|date| daily_view(date, by_date.get(&date).copied()))
        .collect()
}

fn daily_view(date: Date, totals: Option<&UsageTotals>) -> DailyView {
    DailyView {
        date,
        total_tokens: totals.map_or(0, |t| t.tokens.total().0),
        cost_usd_micros: totals.map_or(0, |t| t.cost.0),
        partial: totals.is_some_and(UsageTotals::is_partial),
    }
}

fn model_view(usage: &ModelUsage) -> ModelView {
    ModelView {
        model: usage.model.clone(),
        total_tokens: usage.totals.tokens.total().0,
        cost_usd_micros: usage.totals.cost.0,
        partial: usage.totals.is_partial(),
    }
}
