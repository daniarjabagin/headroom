use headroom_core::quota::{Balance, BalanceAmount, QuotaWindow, WindowId};
use headroom_core::units::{MicroUsd, Percent};
use jiff::tz::TimeZone;
use jiff::{SignedDuration, Timestamp};
use serde_json::{Number, Value};

use super::auth::title_case_word;
use super::number::whole_number;
use super::raw::{RawExtraUsage, RawLimit, RawUsage, RawWindow};

const SCOPED_WEEKLY_PREFIX: &str = "seven_day_";
const MICRO_DIGITS: u32 = 6;
const DEFAULT_CURRENCY_DIGITS: u32 = 2;
const EPOCH_SECONDS_LIMIT: i64 = 10_000_000_000;

#[derive(Debug, Clone, PartialEq)]
pub(super) struct MappedUsage {
    pub(super) windows: Vec<QuotaWindow>,
    pub(super) balances: Vec<Balance>,
}

pub(super) fn map_usage(raw: &RawUsage) -> MappedUsage {
    let mut windows = WindowList::default();
    windows.push(
        raw.five_hour
            .as_ref()
            .and_then(|w| fixed_window(WindowId::Session, "Session", w, WindowId::SESSION_PERIOD)),
    );
    windows.push(
        raw.seven_day
            .as_ref()
            .and_then(|w| fixed_window(WindowId::Weekly, "Weekly", w, WindowId::WEEKLY_PERIOD)),
    );
    for (key, value) in &raw.other {
        if let Some(name) = key.strip_prefix(SCOPED_WEEKLY_PREFIX) {
            windows.push(scoped_weekly_window(name, value));
        }
    }
    for value in raw.limits.iter().flatten() {
        windows.push(limit_window(value));
    }
    MappedUsage {
        windows: windows.0,
        balances: extra_usage_balances(raw.extra_usage.as_ref()),
    }
}

#[derive(Default)]
struct WindowList(Vec<QuotaWindow>);

impl WindowList {
    fn push(&mut self, window: Option<QuotaWindow>) {
        if let Some(window) = window
            && !self.0.iter().any(|known| known.id == window.id)
        {
            self.0.push(window);
        }
    }
}

fn fixed_window(
    id: WindowId,
    label: &str,
    raw: &RawWindow,
    period: SignedDuration,
) -> Option<QuotaWindow> {
    Some(QuotaWindow {
        id,
        label: label.to_owned(),
        used: used_percent(raw.utilization)?,
        resets_at: raw.resets_at.as_ref().and_then(reset_time),
        period: Some(period),
    })
}

fn scoped_weekly_window(name: &str, value: &Value) -> Option<QuotaWindow> {
    let raw: RawWindow = serde_json::from_value(value.clone()).ok()?;
    fixed_window(
        WindowId::Model(slug(name)?),
        &readable(name),
        &raw,
        WindowId::WEEKLY_PERIOD,
    )
}

fn limit_window(value: &Value) -> Option<QuotaWindow> {
    let limit: RawLimit = serde_json::from_value(value.clone()).ok()?;
    let kind = limit.kind.as_deref().filter(|kind| !kind.is_empty())?;
    let (id, label) = limit_identity(kind, &limit)?;
    let period = limit_period(&id, limit.group.as_deref());
    Some(QuotaWindow {
        used: used_percent(limit.percent)?,
        resets_at: limit.resets_at.as_ref().and_then(reset_time),
        id,
        label,
        period,
    })
}

fn limit_identity(kind: &str, limit: &RawLimit) -> Option<(WindowId, String)> {
    let model = limit
        .scope
        .as_ref()
        .and_then(|scope| scope.model.as_ref())
        .and_then(|model| model.display_name.as_deref())
        .map(str::trim)
        .filter(|name| !name.is_empty());
    if let Some(name) = model {
        return Some((WindowId::Model(slug(name)?), name.to_owned()));
    }
    Some(match kind {
        "session" => (WindowId::Session, "Session".to_owned()),
        "weekly_all" => (WindowId::Weekly, "Weekly".to_owned()),
        other => (WindowId::Other(other.to_owned()), readable(other)),
    })
}

fn limit_period(id: &WindowId, group: Option<&str>) -> Option<SignedDuration> {
    match (id, group) {
        (WindowId::Session, _) | (_, Some("session")) => Some(WindowId::SESSION_PERIOD),
        (WindowId::Weekly, _) | (_, Some("weekly")) => Some(WindowId::WEEKLY_PERIOD),
        _ => None,
    }
}

fn used_percent(value: Option<f64>) -> Option<Percent> {
    value.filter(|v| v.is_finite()).map(Percent::new)
}

fn slug(name: &str) -> Option<String> {
    let slug = name
        .split(|c: char| c.is_whitespace() || c == '_')
        .filter(|part| !part.is_empty())
        .map(str::to_lowercase)
        .collect::<Vec<_>>()
        .join("_");
    (!slug.is_empty()).then_some(slug)
}

fn readable(name: &str) -> String {
    name.split('_')
        .filter_map(title_case_word)
        .collect::<Vec<_>>()
        .join(" ")
}

pub(super) fn reset_time(value: &Value) -> Option<Timestamp> {
    match value {
        Value::String(text) => parse_time_text(text.trim()),
        Value::Number(number) => epoch_time(number),
        _ => None,
    }
}

fn parse_time_text(text: &str) -> Option<Timestamp> {
    text.parse::<Timestamp>().ok().or_else(|| {
        let civil: jiff::civil::DateTime = text.parse().ok()?;
        civil
            .to_zoned(TimeZone::UTC)
            .ok()
            .map(|zoned| zoned.timestamp())
    })
}

fn epoch_time(number: &Number) -> Option<Timestamp> {
    let value = whole_number(number)?;
    if value.abs() < EPOCH_SECONDS_LIMIT {
        Timestamp::from_second(value).ok()
    } else {
        Timestamp::from_millisecond(value).ok()
    }
}

fn extra_usage_balances(extra: Option<&RawExtraUsage>) -> Vec<Balance> {
    let Some(extra) = extra.filter(|extra| extra.is_enabled == Some(true)) else {
        return Vec::new();
    };
    if !is_usd(extra.currency.as_deref()) {
        return Vec::new();
    }
    let digits = extra.decimal_places.unwrap_or(DEFAULT_CURRENCY_DIGITS);
    let spent = extra
        .used_credits
        .as_ref()
        .and_then(|a| minor_to_micro(a, digits));
    let limit = extra
        .monthly_limit
        .as_ref()
        .and_then(|a| minor_to_micro(a, digits))
        .filter(|limit| limit.0 > 0);
    [
        ("extra_usage_spent", "Extra usage spent", spent),
        ("extra_usage_limit", "Extra usage limit", limit),
    ]
    .into_iter()
    .filter_map(|(id, label, amount)| {
        Some(Balance {
            id: id.to_owned(),
            label: label.to_owned(),
            amount: BalanceAmount::Usd(amount?),
        })
    })
    .collect()
}

fn is_usd(currency: Option<&str>) -> bool {
    currency.is_none_or(|code| code.eq_ignore_ascii_case("usd"))
}

fn minor_to_micro(amount: &Number, digits: u32) -> Option<MicroUsd> {
    let scale = 10_i64.checked_pow(MICRO_DIGITS.checked_sub(digits)?)?;
    whole_number(amount)?.checked_mul(scale).map(MicroUsd)
}

#[cfg(test)]
#[path = "mapper_tests.rs"]
mod tests;
