use std::collections::{BTreeMap, BTreeSet};

use jiff::civil::Date;
use jiff::tz::TimeZone;
use jiff::{Timestamp, ToSpan};
use serde::{Deserialize, Serialize};

use crate::event::{ServiceTier, UsageEvent};
use crate::tokens::TokenCounts;
use crate::units::{MicroUsd, Tokens};

pub trait PriceBook: Send + Sync {
    fn cost(
        &self,
        model: &str,
        tier: ServiceTier,
        tokens: &TokenCounts,
        web_search: u32,
    ) -> Option<MicroUsd>;
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
pub struct UsageSummary {
    pub today: UsageTotals,
    pub yesterday: UsageTotals,
    pub last_30_days: UsageTotals,
    pub daily: Vec<(Date, UsageTotals)>,
    pub models: Vec<ModelUsage>,
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
            let cost = prices.cost(
                &event.model,
                event.tier,
                &event.tokens,
                event.web_search_requests,
            );
            builder.record(&days, date, event, cost);
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
    today: UsageTotals,
    yesterday: UsageTotals,
    last_30_days: UsageTotals,
    daily: BTreeMap<Date, UsageTotals>,
    models: BTreeMap<String, UsageTotals>,
}

impl SummaryBuilder {
    fn record(&mut self, days: &DayRange, date: Date, event: &UsageEvent, cost: Option<MicroUsd>) {
        self.last_30_days.record(event, cost);
        self.daily.entry(date).or_default().record(event, cost);
        self.models
            .entry(event.model.clone())
            .or_default()
            .record(event, cost);
        if date == days.today {
            self.today.record(event, cost);
        } else if Some(date) == days.yesterday {
            self.yesterday.record(event, cost);
        }
    }

    fn finish(self) -> UsageSummary {
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
        UsageSummary {
            today: self.today,
            yesterday: self.yesterday,
            last_30_days: self.last_30_days,
            daily: self.daily.into_iter().collect(),
            models,
        }
    }
}

#[cfg(test)]
#[path = "usage_tests.rs"]
mod tests;
