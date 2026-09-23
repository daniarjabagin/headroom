use std::collections::{BTreeMap, BTreeSet};

use jiff::civil::Date;
use jiff::tz::TimeZone;
use jiff::{Timestamp, ToSpan};
use serde::{Deserialize, Serialize};

use crate::event::UsageEvent;
use crate::tokens::TokenCounts;
use crate::units::{MicroUsd, Tokens};

pub trait PriceBook: Send + Sync {
    fn cost(&self, event: &UsageEvent) -> Option<MicroUsd>;
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct UsageTotals {
    pub tokens: TokenCounts,
    pub cost: MicroUsd,
    pub unpriced_tokens: Tokens,
    pub unpriced_models: BTreeSet<String>,
}

impl UsageTotals {
    #[must_use]
    pub fn is_partial(&self) -> bool {
        !self.unpriced_models.is_empty()
    }

    fn record(&mut self, event: &UsageEvent, cost: Option<MicroUsd>) {
        self.tokens += event.tokens;
        if let Some(cost) = cost {
            self.cost += cost;
        } else {
            self.unpriced_tokens += event.tokens.total();
            self.unpriced_models.insert(event.model.clone());
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelUsage {
    pub model: String,
    pub totals: UsageTotals,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PeriodUsage {
    pub totals: UsageTotals,
    pub models: Vec<ModelUsage>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct UsageSummary {
    pub today: PeriodUsage,
    pub yesterday: PeriodUsage,
    pub last_30_days: PeriodUsage,
    pub daily: Vec<(Date, UsageTotals)>,
}

const WINDOW_DAYS: i64 = 30;

#[must_use]
pub fn aggregate(
    events: &[UsageEvent],
    prices: &dyn PriceBook,
    tz: &TimeZone,
    now: Timestamp,
) -> UsageSummary {
    let days = DayRange::ending_at(tz, now);
    let mut builder = SummaryBuilder::default();
    for event in events {
        let date = tz.to_datetime(event.at).date();
        if days.contains(date) {
            builder.record(&days, date, event, prices.cost(event));
        }
    }
    builder.finish()
}

struct DayRange {
    first: Date,
    yesterday: Option<Date>,
    today: Date,
}

impl DayRange {
    fn ending_at(tz: &TimeZone, now: Timestamp) -> DayRange {
        let today = tz.to_datetime(now).date();
        DayRange {
            first: today
                .checked_sub((WINDOW_DAYS - 1).days())
                .unwrap_or(Date::MIN),
            yesterday: today.yesterday().ok(),
            today,
        }
    }

    fn contains(&self, date: Date) -> bool {
        self.first <= date && date <= self.today
    }
}

#[derive(Default)]
struct SummaryBuilder {
    today: PeriodBuilder,
    yesterday: PeriodBuilder,
    last_30_days: PeriodBuilder,
    daily: BTreeMap<Date, UsageTotals>,
}

impl SummaryBuilder {
    fn record(&mut self, days: &DayRange, date: Date, event: &UsageEvent, cost: Option<MicroUsd>) {
        self.last_30_days.record(event, cost);
        self.daily.entry(date).or_default().record(event, cost);
        if date == days.today {
            self.today.record(event, cost);
        } else if Some(date) == days.yesterday {
            self.yesterday.record(event, cost);
        }
    }

    fn finish(self) -> UsageSummary {
        UsageSummary {
            today: self.today.finish(),
            yesterday: self.yesterday.finish(),
            last_30_days: self.last_30_days.finish(),
            daily: self.daily.into_iter().collect(),
        }
    }
}

#[derive(Default)]
struct PeriodBuilder {
    totals: UsageTotals,
    models: BTreeMap<String, UsageTotals>,
}

impl PeriodBuilder {
    fn record(&mut self, event: &UsageEvent, cost: Option<MicroUsd>) {
        self.totals.record(event, cost);
        self.models
            .entry(event.model.clone())
            .or_default()
            .record(event, cost);
    }

    fn finish(self) -> PeriodUsage {
        let mut models: Vec<ModelUsage> = self
            .models
            .into_iter()
            .map(|(model, totals)| ModelUsage { model, totals })
            .collect();
        models.sort_by(|a, b| {
            b.totals
                .cost
                .cmp(&a.totals.cost)
                .then_with(|| b.totals.tokens.total().cmp(&a.totals.tokens.total()))
                .then_with(|| a.model.cmp(&b.model))
        });
        PeriodUsage {
            totals: self.totals,
            models,
        }
    }
}

#[cfg(test)]
#[path = "usage_tests.rs"]
mod tests;
