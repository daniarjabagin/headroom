use std::collections::BTreeMap;

use headroom_core::usage::{PeriodUsage, UsageSummary, UsageTotals};
use jiff::ToSpan;
use jiff::civil::Date;

use super::AssembleContext;
use super::models::{model_view, top_models};
use super::payload::{DailyView, TokensView, TotalsView, UsageView};
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
    }
}

fn totals_view(period: &PeriodUsage) -> TotalsView {
    let totals = &period.totals;
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
        models: period.models.iter().map(model_view).collect(),
        models_other: None,
    }
}

#[must_use]
pub fn with_top_models(mut view: UsageView) -> UsageView {
    for totals in [&mut view.today, &mut view.yesterday, &mut view.last_30_days] {
        let (models, other) = top_models(std::mem::take(&mut totals.models));
        totals.models = models;
        totals.models_other = other;
    }
    view
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
