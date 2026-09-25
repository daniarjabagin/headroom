use std::cmp::Ordering;
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

pub const TOP_PROJECTS: usize = 5;

const TOKENS_PER_MTOK: i128 = 1_000_000;

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

    #[must_use]
    pub fn priced_tokens(&self) -> Tokens {
        self.tokens.total().saturating_sub(self.unpriced_tokens)
    }

    /// Blended cost of one million priced tokens, rounded half away from zero; unpriced tokens are left out.
    #[must_use]
    pub fn cost_per_mtok(&self) -> Option<MicroUsd> {
        let priced = i128::from(self.priced_tokens().0);
        if priced == 0 {
            return None;
        }
        let per_mtok = divide_rounded(i128::from(self.cost.0) * TOKENS_PER_MTOK, priced);
        Some(MicroUsd(saturate_to_i64(per_mtok)))
    }

    pub fn absorb(&mut self, other: &UsageTotals) {
        self.tokens += other.tokens;
        self.cost += other.cost;
        self.unpriced_tokens += other.unpriced_tokens;
        self.unpriced_models
            .extend(other.unpriced_models.iter().cloned());
    }

    pub fn record(&mut self, event: &UsageEvent, cost: Option<MicroUsd>) {
        self.tokens += event.tokens;
        if let Some(cost) = cost {
            self.cost += cost;
        } else {
            self.unpriced_tokens += event.tokens.total();
            self.unpriced_models.insert(event.model.clone());
        }
    }
}

fn divide_rounded(numerator: i128, denominator: i128) -> i128 {
    let half = denominator / 2;
    if numerator >= 0 {
        (numerator + half) / denominator
    } else {
        (numerator - half) / denominator
    }
}

fn saturate_to_i64(value: i128) -> i64 {
    i64::try_from(value).unwrap_or(if value < 0 { i64::MIN } else { i64::MAX })
}

#[must_use]
pub fn most_expensive_first(a: &UsageTotals, b: &UsageTotals) -> Ordering {
    b.cost
        .cmp(&a.cost)
        .then_with(|| b.tokens.total().cmp(&a.tokens.total()))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelUsage {
    pub model: String,
    pub totals: UsageTotals,
}

/// Usage of one working directory; `None` collects events whose log names no directory.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectUsage {
    pub project: Option<String>,
    pub totals: UsageTotals,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TopProjects {
    pub top: Vec<ProjectUsage>,
    pub other: Option<UsageTotals>,
}

/// Keeps the `limit` most expensive projects and folds the rest into `other`, so the parts sum to the whole.
#[must_use]
pub fn top_projects(mut projects: Vec<ProjectUsage>, limit: usize) -> TopProjects {
    sort_projects(&mut projects);
    let rest = projects.split_off(limit.min(projects.len()));
    let other = (!rest.is_empty()).then(|| {
        let mut sum = UsageTotals::default();
        for project in &rest {
            sum.absorb(&project.totals);
        }
        sum
    });
    TopProjects {
        top: projects,
        other,
    }
}

fn sort_projects(projects: &mut [ProjectUsage]) {
    projects.sort_by(|a, b| {
        most_expensive_first(&a.totals, &b.totals).then_with(|| a.project.cmp(&b.project))
    });
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PeriodUsage {
    pub totals: UsageTotals,
    pub models: Vec<ModelUsage>,
    /// Every project of the period, most expensive first, uncut so homes can be merged before `top_projects`.
    #[serde(default)]
    pub projects: Vec<ProjectUsage>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct UsageSummary {
    pub today: PeriodUsage,
    pub yesterday: PeriodUsage,
    #[serde(default)]
    pub last_7_days: PeriodUsage,
    pub last_30_days: PeriodUsage,
    pub daily: Vec<(Date, UsageTotals)>,
}

const WEEK_DAYS: i64 = 7;
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
            builder.record(&days, date, event, event_cost(event, prices));
        }
    }
    builder.finish()
}

/// The cost the provider logged for the event, else the price book's; `None` when neither knows it.
#[must_use]
pub fn event_cost(event: &UsageEvent, prices: &dyn PriceBook) -> Option<MicroUsd> {
    event.reported_cost.or_else(|| prices.cost(event))
}

struct DayRange {
    first: Date,
    week_first: Date,
    yesterday: Option<Date>,
    today: Date,
}

impl DayRange {
    fn ending_at(tz: &TimeZone, now: Timestamp) -> DayRange {
        let today = tz.to_datetime(now).date();
        DayRange {
            first: days_before(today, WINDOW_DAYS - 1),
            week_first: days_before(today, WEEK_DAYS - 1),
            yesterday: today.yesterday().ok(),
            today,
        }
    }

    fn contains(&self, date: Date) -> bool {
        self.first <= date && date <= self.today
    }

    fn in_week(&self, date: Date) -> bool {
        self.week_first <= date && date <= self.today
    }
}

fn days_before(day: Date, count: i64) -> Date {
    day.checked_sub(count.days()).unwrap_or(Date::MIN)
}

#[derive(Default)]
struct SummaryBuilder {
    today: PeriodBuilder,
    yesterday: PeriodBuilder,
    last_7_days: PeriodBuilder,
    last_30_days: PeriodBuilder,
    daily: BTreeMap<Date, UsageTotals>,
}

impl SummaryBuilder {
    fn record(&mut self, days: &DayRange, date: Date, event: &UsageEvent, cost: Option<MicroUsd>) {
        self.last_30_days.record(event, cost);
        self.daily.entry(date).or_default().record(event, cost);
        if days.in_week(date) {
            self.last_7_days.record(event, cost);
        }
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
            last_7_days: self.last_7_days.finish(),
            last_30_days: self.last_30_days.finish(),
            daily: self.daily.into_iter().collect(),
        }
    }
}

#[derive(Default)]
struct PeriodBuilder {
    totals: UsageTotals,
    models: BTreeMap<String, UsageTotals>,
    projects: BTreeMap<Option<String>, UsageTotals>,
}

impl PeriodBuilder {
    fn record(&mut self, event: &UsageEvent, cost: Option<MicroUsd>) {
        self.totals.record(event, cost);
        self.models
            .entry(event.model.clone())
            .or_default()
            .record(event, cost);
        self.projects
            .entry(event.project.clone())
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
            most_expensive_first(&a.totals, &b.totals).then_with(|| a.model.cmp(&b.model))
        });
        let mut projects: Vec<ProjectUsage> = self
            .projects
            .into_iter()
            .map(|(project, totals)| ProjectUsage { project, totals })
            .collect();
        sort_projects(&mut projects);
        PeriodUsage {
            totals: self.totals,
            models,
            projects,
        }
    }
}

#[cfg(test)]
#[path = "usage_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "usage_projects_tests.rs"]
mod projects_tests;
