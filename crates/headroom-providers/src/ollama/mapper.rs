use headroom_core::account::AccountIdentity;
use headroom_core::pace::Tone;
use headroom_core::quota::{
    Balance, BalanceAmount, LimitsSnapshot, LimitsSource, Notice, QuotaWindow, WindowId,
};
use headroom_core::units::{MicroUsd, Percent};
use jiff::{SignedDuration, Timestamp};
use serde_json::Value;

use super::money::micro_usd;
use super::raw::{RawLimit, RawMe, RawUsage};

const EXTRA_USAGE_ID: &str = "extra_usage_4w";
const EXTRA_USAGE_LABEL: &str = "Extra usage (4 weeks)";
pub(super) const NO_LIMITS_TEXT: &str = "Ollama reports no Cloud limits for this account yet.";
pub(super) const NO_PLAN_TEXT: &str =
    "Could not read the Ollama plan; the usage above is up to date.";

pub(super) fn map_snapshot(
    usage: &RawUsage,
    me: Option<&RawMe>,
    stable_key: String,
    now: Timestamp,
) -> LimitsSnapshot {
    let windows = windows(usage);
    let mut notices = Vec::new();
    if windows.is_empty() {
        notices.push(notice(NO_LIMITS_TEXT));
    }
    if me.is_none() {
        notices.push(notice(NO_PLAN_TEXT));
    }
    LimitsSnapshot {
        identity: identity(me, stable_key),
        windows,
        balances: extra_usage(usage).into_iter().collect(),
        notices,
        fetched_at: now,
        source: LimitsSource::Live,
    }
}

fn windows(usage: &RawUsage) -> Vec<QuotaWindow> {
    let limits = &usage.limits;
    [
        window(
            limits.session.as_ref(),
            WindowId::Session,
            "Session",
            Some(WindowId::SESSION_PERIOD),
        ),
        window(
            limits.weekly.as_ref(),
            WindowId::Weekly,
            "Weekly",
            Some(WindowId::WEEKLY_PERIOD),
        ),
        window(
            limits.monthly.as_ref(),
            WindowId::Other("monthly".into()),
            "Monthly",
            None,
        ),
    ]
    .into_iter()
    .flatten()
    .collect()
}

fn window(
    limit: Option<&RawLimit>,
    id: WindowId,
    label: &str,
    period: Option<SignedDuration>,
) -> Option<QuotaWindow> {
    let fraction = limit?.usage.filter(|value| value.is_finite())?;
    Some(QuotaWindow {
        id,
        label: label.to_owned(),
        used: Percent::new(fraction * 100.0),
        resets_at: None,
        period,
    })
}

fn extra_usage(usage: &RawUsage) -> Option<Balance> {
    let cost = usage.activity.as_ref()?.cost.as_ref()?;
    let amount = cost_micro_usd(cost);
    if amount.is_none() {
        tracing::warn!("ollama activity cost is not a plain decimal amount; not shown");
    }
    Some(Balance {
        id: EXTRA_USAGE_ID.to_owned(),
        label: EXTRA_USAGE_LABEL.to_owned(),
        amount: BalanceAmount::Usd(amount?),
    })
}

fn cost_micro_usd(cost: &Value) -> Option<MicroUsd> {
    match cost {
        Value::String(text) => micro_usd(text),
        Value::Number(number) => micro_usd(&number.to_string()),
        _ => None,
    }
}

fn identity(me: Option<&RawMe>, stable_key: String) -> AccountIdentity {
    AccountIdentity {
        email: me.and_then(|me| non_empty(me.email.as_deref())),
        plan: me.and_then(|me| non_empty(me.plan.as_deref()).map(|plan| title_case(&plan))),
        stable_key,
    }
}

fn non_empty(text: Option<&str>) -> Option<String> {
    text.map(str::trim)
        .filter(|text| !text.is_empty())
        .map(str::to_owned)
}

fn title_case(text: &str) -> String {
    let mut chars = text.chars();
    chars.next().map_or_else(String::new, |first| {
        first
            .to_uppercase()
            .chain(chars.flat_map(char::to_lowercase))
            .collect()
    })
}

pub(super) fn notice(text: &str) -> Notice {
    Notice {
        tone: Tone::Warning,
        text: text.to_owned(),
    }
}

#[cfg(test)]
#[path = "mapper_tests.rs"]
mod tests;
