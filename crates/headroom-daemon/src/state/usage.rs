use std::collections::BTreeMap;

use headroom_core::usage::{PeriodUsage, UsageSummary, UsageTotals};
use jiff::ToSpan;
use jiff::civil::Date;

use super::AssembleContext;
use super::models::top_models;
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
        provider: home.provider.clone(),
        provider_name: ctx.catalog.display_name(&home.provider).to_owned(),
        usage_home: ctx.homes.show(&home.home),
        today: totals_view(&summary.today),
        yesterday: totals_view(&summary.yesterday),
        last_30_days: totals_view(&summary.last_30_days),
        daily: daily(summary, today),
    }
}

fn totals_view(period: &PeriodUsage) -> TotalsView {
    let totals = &period.totals;
    let (models, models_other) = top_models(&period.models);
    TotalsView {
        tokens: TokensView::of(&totals.tokens),
        cost_usd_micros: totals.cost.0,
        partial: totals.is_partial(),
        unpriced_tokens: totals.unpriced_tokens.0,
        unpriced_models: totals.unpriced_models.iter().cloned().collect(),
        models,
        models_other,
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
