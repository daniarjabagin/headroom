use headroom_core::account::ProviderId;
use jiff::civil::Date;
use jiff::tz::TimeZone;
use jiff::{Timestamp, ToSpan};
use serde::Deserialize;

use crate::catalog::ProviderCatalog;
use crate::error::SpendQueryError;
use crate::usage::breakdown::{GroupBy, SpendRange};
use crate::usage::summary::RETENTION;

const SECONDS_PER_DAY: i64 = 24 * 60 * 60;
const DST_MARGIN_DAYS: i64 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum Period {
    #[serde(rename = "today")]
    Today,
    #[serde(rename = "yesterday")]
    Yesterday,
    #[serde(rename = "7d")]
    Week,
    #[serde(rename = "30d")]
    Month,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawRequest {
    #[serde(default)]
    period: Option<Period>,
    #[serde(default)]
    since: Option<Date>,
    #[serde(default)]
    until: Option<Date>,
    by: GroupBy,
    #[serde(default)]
    provider: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpendRequest {
    pub since: Date,
    pub until: Date,
    pub by: GroupBy,
    pub provider: Option<ProviderId>,
    pub range: SpendRange,
}

pub fn resolve(
    json: &str,
    tz: &TimeZone,
    now: Timestamp,
    catalog: &ProviderCatalog,
) -> Result<SpendRequest, SpendQueryError> {
    let raw: RawRequest = serde_json::from_str(json)?;
    let today = tz.to_datetime(now).date();
    let (since, until) = dates(&raw, today)?;
    check_dates(since, until, today)?;
    Ok(SpendRequest {
        since,
        until,
        by: raw.by,
        provider: raw.provider.map(|id| known(id, catalog)).transpose()?,
        range: SpendRange::local_dates(tz, since, until)?,
    })
}

fn dates(raw: &RawRequest, today: Date) -> Result<(Date, Date), SpendQueryError> {
    match (raw.period, raw.since, raw.until) {
        (Some(_), Some(_), _) => Err(SpendQueryError::PeriodAndSince),
        (_, None, Some(_)) => Err(SpendQueryError::UntilWithoutSince),
        (Some(period), None, None) => period_dates(period, today),
        (None, Some(since), until) => Ok((since, until.unwrap_or(today))),
        (None, None, None) => Err(SpendQueryError::MissingRange),
    }
}

fn period_dates(period: Period, today: Date) -> Result<(Date, Date), SpendQueryError> {
    Ok(match period {
        Period::Today => (today, today),
        Period::Yesterday => {
            let yesterday = today.yesterday()?;
            (yesterday, yesterday)
        }
        Period::Week => (today.checked_sub(6.days())?, today),
        Period::Month => (today.checked_sub(29.days())?, today),
    })
}

fn check_dates(since: Date, until: Date, today: Date) -> Result<(), SpendQueryError> {
    if since > today {
        return Err(SpendQueryError::Future(since));
    }
    if until > today {
        return Err(SpendQueryError::Future(until));
    }
    if until < since {
        return Err(SpendQueryError::UntilBeforeSince { since, until });
    }
    let earliest = earliest_since(today)?;
    if since < earliest {
        return Err(SpendQueryError::BeyondRetention(earliest));
    }
    Ok(())
}

fn earliest_since(today: Date) -> Result<Date, SpendQueryError> {
    let retained_days = RETENTION.as_secs() / SECONDS_PER_DAY;
    Ok(today.checked_sub((retained_days - 1 - DST_MARGIN_DAYS).days())?)
}

fn known(id: String, catalog: &ProviderCatalog) -> Result<ProviderId, SpendQueryError> {
    let listed = catalog
        .payload()
        .providers
        .into_iter()
        .find(|provider| provider.id.as_str() == id);
    listed
        .map(|provider| provider.id)
        .ok_or(SpendQueryError::UnknownProvider(id))
}

#[cfg(test)]
#[path = "request_tests.rs"]
mod tests;
