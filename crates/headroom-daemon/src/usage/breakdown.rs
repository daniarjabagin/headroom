use std::collections::{BTreeMap, BTreeSet};
use std::num::NonZeroU32;

use headroom_core::account::ProviderId;
use headroom_core::event::UsageEvent;
use headroom_core::usage::{PriceBook, UsageTotals, event_cost, most_expensive_first};
use jiff::civil::Date;
use jiff::tz::TimeZone;
use jiff::{Timestamp, ToSpan};
use rusqlite::Connection;

use crate::error::StorageError;
use crate::home::UsageHome;
use crate::storage::events;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupBy {
    Model,
    Project,
    Provider,
    Day,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum GroupKey {
    Model(String),
    Project(Option<String>),
    Provider(ProviderId),
    Day(Date),
}

/// Half-open interval `[since, until)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpendRange {
    pub since: Timestamp,
    pub until: Timestamp,
}

impl SpendRange {
    /// The last `days` calendar days in `tz`, today included, from local midnight to local midnight.
    pub fn local_days(
        tz: &TimeZone,
        now: Timestamp,
        days: NonZeroU32,
    ) -> Result<SpendRange, jiff::Error> {
        let today = tz.to_datetime(now).date();
        let first = today.checked_sub(i64::from(days.get() - 1).days())?;
        Ok(SpendRange {
            since: local_midnight(first, tz)?,
            until: local_midnight(today.tomorrow()?, tz)?,
        })
    }

    fn contains(&self, at: Timestamp) -> bool {
        self.since <= at && at < self.until
    }
}

fn local_midnight(day: Date, tz: &TimeZone) -> Result<Timestamp, jiff::Error> {
    Ok(day.to_zoned(tz.clone())?.timestamp())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpendRow {
    pub key: GroupKey,
    pub totals: UsageTotals,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Breakdown {
    pub totals: UsageTotals,
    pub rows: Vec<SpendRow>,
}

pub struct HomeEvents {
    pub provider: ProviderId,
    pub events: Vec<UsageEvent>,
}

pub struct SpendQuery<'a> {
    pub prices: &'a dyn PriceBook,
    pub tz: &'a TimeZone,
    pub range: SpendRange,
    pub group_by: GroupBy,
}

pub fn query(
    conn: &Connection,
    homes: &BTreeSet<UsageHome>,
    spend: &SpendQuery<'_>,
) -> Result<Breakdown, StorageError> {
    let mut loaded = Vec::with_capacity(homes.len());
    for home in homes {
        loaded.push(HomeEvents {
            provider: home.provider.clone(),
            events: events::load_since(conn, home, spend.range.since)?,
        });
    }
    Ok(breakdown(&loaded, spend))
}

#[must_use]
pub fn breakdown(homes: &[HomeEvents], spend: &SpendQuery<'_>) -> Breakdown {
    let mut totals = UsageTotals::default();
    let mut groups: BTreeMap<GroupKey, UsageTotals> = BTreeMap::new();
    for home in homes {
        for event in home.events.iter().filter(|e| spend.range.contains(e.at)) {
            let cost = event_cost(event, spend.prices);
            totals.record(event, cost);
            let key = group_key(spend, &home.provider, event);
            groups.entry(key).or_default().record(event, cost);
        }
    }
    Breakdown {
        totals,
        rows: sorted_rows(groups, spend.group_by),
    }
}

fn group_key(spend: &SpendQuery<'_>, provider: &ProviderId, event: &UsageEvent) -> GroupKey {
    match spend.group_by {
        GroupBy::Model => GroupKey::Model(event.model.clone()),
        GroupBy::Project => GroupKey::Project(event.project.clone()),
        GroupBy::Provider => GroupKey::Provider(provider.clone()),
        GroupBy::Day => GroupKey::Day(spend.tz.to_datetime(event.at).date()),
    }
}

fn sorted_rows(groups: BTreeMap<GroupKey, UsageTotals>, group_by: GroupBy) -> Vec<SpendRow> {
    let mut rows: Vec<SpendRow> = groups
        .into_iter()
        .map(|(key, totals)| SpendRow { key, totals })
        .collect();
    if group_by != GroupBy::Day {
        rows.sort_by(|a, b| {
            most_expensive_first(&a.totals, &b.totals).then_with(|| a.key.cmp(&b.key))
        });
    }
    rows
}

#[cfg(test)]
#[path = "breakdown_tests.rs"]
mod tests;
