use headroom_core::provider::ProviderError;
use headroom_core::quota::{Balance, BalanceAmount, QuotaWindow, WindowId};
use headroom_core::units::{MicroUsd, Percent};
use jiff::{SignedDuration, Timestamp};
use serde_json::Value;

use super::raw::{RawPlanInfo, RawPlanStatus, RawUserStatus};

const DAY: SignedDuration = SignedDuration::from_hours(24);
const NO_PLAN: &str = "No active Devin plan.";

#[derive(Debug, Clone, PartialEq)]
pub(super) struct MappedStatus {
    pub(super) plan: Option<String>,
    pub(super) email: Option<String>,
    pub(super) windows: Vec<QuotaWindow>,
    pub(super) balances: Vec<Balance>,
}

pub(super) fn map_status(raw: &RawUserStatus) -> Result<MappedStatus, ProviderError> {
    let status = raw.plan_status.clone().unwrap_or_default();
    let info = status.plan_info.clone().unwrap_or_default();
    let mut windows = Vec::new();
    if !hides_daily(&info)? {
        windows.extend(daily_window(&status)?);
    }
    windows.extend(weekly_window(&status)?);
    let balances = overage_balance(&status)?.into_iter().collect::<Vec<_>>();
    if windows.is_empty() && balances.is_empty() {
        return Err(no_subscription());
    }
    Ok(MappedStatus {
        plan: non_empty(info.plan_name.as_deref()),
        email: non_empty(raw.email.as_deref()),
        windows,
        balances,
    })
}

pub(super) fn no_subscription() -> ProviderError {
    ProviderError::NoSubscription {
        detail: NO_PLAN.to_owned(),
    }
}

fn hides_daily(info: &RawPlanInfo) -> Result<bool, ProviderError> {
    Ok(field(info.hide_daily_quota.as_ref(), "hideDailyQuota", flag)?.unwrap_or(false))
}

fn daily_window(status: &RawPlanStatus) -> Result<Option<QuotaWindow>, ProviderError> {
    let remaining = field(
        status.daily_quota_remaining_percent.as_ref(),
        "dailyQuotaRemainingPercent",
        decimal,
    )?;
    let resets_at = reset_time(
        status.daily_quota_reset_at_unix.as_ref(),
        "dailyQuotaResetAtUnix",
    )?;
    Ok(remaining.map(|remaining| QuotaWindow {
        id: WindowId::Other("daily".to_owned()),
        label: "Daily".to_owned(),
        used: used_from_remaining(remaining),
        resets_at,
        period: Some(DAY),
    }))
}

fn weekly_window(status: &RawPlanStatus) -> Result<Option<QuotaWindow>, ProviderError> {
    let remaining = field(
        status.weekly_quota_remaining_percent.as_ref(),
        "weeklyQuotaRemainingPercent",
        decimal,
    )?;
    let resets_at = reset_time(
        status.weekly_quota_reset_at_unix.as_ref(),
        "weeklyQuotaResetAtUnix",
    )?;
    let remaining = match (remaining, resets_at) {
        (Some(remaining), _) => remaining,
        (None, Some(_)) => 0.0,
        (None, None) => return Ok(None),
    };
    Ok(Some(QuotaWindow {
        id: WindowId::Weekly,
        label: "Weekly".to_owned(),
        used: used_from_remaining(remaining),
        resets_at,
        period: Some(WindowId::WEEKLY_PERIOD),
    }))
}

fn overage_balance(status: &RawPlanStatus) -> Result<Option<Balance>, ProviderError> {
    let micros = field(
        status.overage_balance_micros.as_ref(),
        "overageBalanceMicros",
        integer,
    )?;
    Ok(micros.map(|micros| Balance {
        id: "extra_usage".to_owned(),
        label: "Extra usage balance".to_owned(),
        amount: BalanceAmount::Usd(MicroUsd(micros)),
    }))
}

fn used_from_remaining(remaining: f64) -> Percent {
    Percent::new(100.0 - remaining)
}

fn reset_time(value: Option<&Value>, name: &str) -> Result<Option<Timestamp>, ProviderError> {
    match field(value, name, integer)? {
        None | Some(0) => Ok(None),
        Some(seconds) => Timestamp::from_second(seconds)
            .map(Some)
            .map_err(|_| invalid(name)),
    }
}

fn field<T>(
    value: Option<&Value>,
    name: &str,
    parse: fn(&Value) -> Option<T>,
) -> Result<Option<T>, ProviderError> {
    match value {
        None | Some(Value::Null) => Ok(None),
        Some(value) => parse(value).map(Some).ok_or_else(|| invalid(name)),
    }
}

fn invalid(name: &str) -> ProviderError {
    ProviderError::InvalidResponse(format!("Devin user status has an unreadable {name}"))
}

fn decimal(value: &Value) -> Option<f64> {
    let parsed = match value {
        Value::Number(number) => number.as_f64(),
        Value::String(text) => text.trim().parse::<f64>().ok(),
        _ => None,
    };
    parsed.filter(|number| number.is_finite())
}

fn integer(value: &Value) -> Option<i64> {
    match value {
        Value::Number(number) => number.as_i64(),
        Value::String(text) => text.trim().parse::<i64>().ok(),
        _ => None,
    }
}

fn flag(value: &Value) -> Option<bool> {
    match value {
        Value::Bool(flag) => Some(*flag),
        Value::String(text) => text.trim().parse::<bool>().ok(),
        Value::Number(number) => number.as_u64().filter(|n| *n <= 1).map(|n| n == 1),
        _ => None,
    }
}

fn non_empty(text: Option<&str>) -> Option<String> {
    text.map(str::trim)
        .filter(|text| !text.is_empty())
        .map(str::to_owned)
}

#[cfg(test)]
#[path = "mapper_tests.rs"]
mod tests;
