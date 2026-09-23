use headroom_core::provider::ProviderError;
use headroom_core::quota::{Balance, BalanceAmount, QuotaWindow, WindowId};
use headroom_core::units::{MicroUsd, Percent};
use jiff::{SignedDuration, Timestamp};
use serde_json::Value;

use super::number::{Decimal, epoch_millis};
use super::raw::{RawPeriodUsage, RawPlanUsage, RawSpendLimitUsage};

const TEAM: &str = "team";

#[derive(Debug, Clone, Default, PartialEq)]
pub(super) struct MappedUsage {
    pub(super) windows: Vec<QuotaWindow>,
    pub(super) balances: Vec<Balance>,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) enum PlanUsage {
    Mapped(MappedUsage),
    RequestBased,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Cycle {
    pub(super) resets_at: Option<Timestamp>,
    pub(super) period: Option<SignedDuration>,
}

pub(super) fn map_period_usage(
    raw: &RawPeriodUsage,
    plan: Option<&str>,
) -> Result<PlanUsage, ProviderError> {
    let plan_usage = raw
        .plan_usage
        .as_ref()
        .filter(|_| raw.enabled != Some(false))
        .ok_or_else(|| no_subscription(plan))?;
    let has_limit = decimal(plan_usage.limit.as_ref()).is_some();
    let has_percent = decimal(plan_usage.total_percent_used.as_ref()).is_some();
    if !has_limit && !has_percent {
        return Ok(PlanUsage::RequestBased);
    }
    let cycle = billing_cycle(raw);
    let team = is_team(plan, raw.spend_limit_usage.as_ref());
    let windows = [
        total_percent(plan_usage, team).map(|used| window("total", "Total", used, cycle)),
        percent(plan_usage.auto_percent_used.as_ref())
            .map(|used| window("auto", "Auto", used, cycle)),
        percent(plan_usage.api_percent_used.as_ref()).map(|used| window("api", "API", used, cycle)),
    ];
    Ok(PlanUsage::Mapped(MappedUsage {
        windows: windows.into_iter().flatten().collect(),
        balances: raw
            .spend_limit_usage
            .as_ref()
            .map(on_demand_balances)
            .unwrap_or_default(),
    }))
}

pub(super) fn no_subscription(plan: Option<&str>) -> ProviderError {
    let detail = match plan.and_then(plan_label) {
        Some(plan) => format!("No active Cursor subscription ({plan} plan)."),
        None => "No active Cursor subscription.".to_owned(),
    };
    ProviderError::NoSubscription { detail }
}

pub(super) fn plan_label(plan: &str) -> Option<String> {
    let words: Vec<String> = plan
        .split(|c: char| c.is_whitespace() || c == '_' || c == '-')
        .filter_map(title_case)
        .collect();
    (!words.is_empty()).then(|| words.join(" "))
}

fn title_case(word: &str) -> Option<String> {
    let mut chars = word.chars();
    let first = chars.next()?;
    Some(
        first
            .to_uppercase()
            .chain(chars.flat_map(char::to_lowercase))
            .collect(),
    )
}

pub(super) fn window(id: &str, label: &str, used: Percent, cycle: Cycle) -> QuotaWindow {
    QuotaWindow {
        id: WindowId::Other(id.to_owned()),
        label: label.to_owned(),
        used,
        resets_at: cycle.resets_at,
        period: cycle.period,
    }
}

pub(super) fn cycle_between(start: Option<Timestamp>, end: Option<Timestamp>) -> Cycle {
    let period = start
        .zip(end)
        .filter(|(start, end)| end > start)
        .map(|(start, end)| end.duration_since(start));
    Cycle {
        resets_at: end,
        period,
    }
}

fn billing_cycle(raw: &RawPeriodUsage) -> Cycle {
    let millis = |value: Option<&Value>| value.and_then(epoch_millis);
    cycle_between(
        millis(raw.billing_cycle_start.as_ref()),
        millis(raw.billing_cycle_end.as_ref()),
    )
}

fn is_team(plan: Option<&str>, spend: Option<&RawSpendLimitUsage>) -> bool {
    let named_team = plan.is_some_and(|plan| plan.trim().eq_ignore_ascii_case(TEAM));
    let team_limit = spend
        .and_then(|spend| spend.limit_type.as_deref())
        .is_some_and(|kind| kind.eq_ignore_ascii_case(TEAM));
    let pooled = spend
        .and_then(|spend| decimal(spend.pooled_limit.as_ref()))
        .is_some_and(Decimal::is_positive);
    named_team || team_limit || pooled
}

fn total_percent(plan_usage: &RawPlanUsage, team: bool) -> Option<Percent> {
    let reported = decimal(plan_usage.total_percent_used.as_ref()).map(Decimal::to_f64);
    let spent = spent_share(plan_usage);
    let used = if team {
        spent.or(reported)
    } else {
        reported.or(spent)
    };
    used.filter(|value| value.is_finite()).map(Percent::new)
}

fn spent_share(plan_usage: &RawPlanUsage) -> Option<f64> {
    let limit = decimal(plan_usage.limit.as_ref())
        .filter(|limit| limit.is_positive())?
        .to_f64();
    let spent = match decimal(plan_usage.total_spend.as_ref()) {
        Some(spent) => spent.to_f64(),
        None => limit - decimal(plan_usage.remaining.as_ref())?.to_f64(),
    };
    Some(spent / limit * 100.0)
}

fn percent(value: Option<&Value>) -> Option<Percent> {
    decimal(value)
        .map(Decimal::to_f64)
        .filter(|value| value.is_finite())
        .map(Percent::new)
}

fn on_demand_balances(usage: &RawSpendLimitUsage) -> Vec<Balance> {
    let cents =
        |value: &Option<Value>| decimal(value.as_ref()).and_then(Decimal::cents_to_micro_usd);
    let limit = cents(&usage.individual_limit)
        .or_else(|| cents(&usage.pooled_limit))
        .unwrap_or(MicroUsd::ZERO);
    let remaining = cents(&usage.individual_remaining)
        .or_else(|| cents(&usage.pooled_remaining))
        .unwrap_or(MicroUsd::ZERO);
    let reported: Vec<MicroUsd> = [
        &usage.individual_used,
        &usage.pooled_used,
        &usage.total_spend,
    ]
    .into_iter()
    .filter_map(cents)
    .collect();
    let spent = on_demand_spent(&reported, limit, remaining);
    let limit = (limit.0 > 0).then_some(limit);
    let spent = (limit.is_some() || spent.0 > 0).then_some(spent);
    [
        ("on_demand_spent", "On-demand spent", spent),
        ("on_demand_limit", "On-demand limit", limit),
    ]
    .into_iter()
    .filter_map(|(id, label, amount)| Some(usd_balance(id, label, amount?)))
    .collect()
}

fn on_demand_spent(reported: &[MicroUsd], limit: MicroUsd, remaining: MicroUsd) -> MicroUsd {
    if let Some(positive) = reported.iter().find(|amount| amount.0 > 0) {
        return *positive;
    }
    let inferred = limit.0.saturating_sub(remaining.0);
    if inferred > 0 {
        return MicroUsd(inferred);
    }
    reported.first().copied().unwrap_or(MicroUsd::ZERO)
}

pub(super) fn usd_balance(id: &str, label: &str, amount: MicroUsd) -> Balance {
    Balance {
        id: id.to_owned(),
        label: label.to_owned(),
        amount: BalanceAmount::Usd(amount),
    }
}

pub(super) fn decimal(value: Option<&Value>) -> Option<Decimal> {
    value.and_then(Decimal::parse)
}

#[cfg(test)]
#[path = "mapper_tests.rs"]
mod tests;
