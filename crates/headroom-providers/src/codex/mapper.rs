use headroom_core::account::AccountIdentity;
use headroom_core::quota::{Balance, BalanceAmount, LimitsSnapshot, LimitsSource, QuotaWindow};
use headroom_core::units::Percent;
use jiff::{SignedDuration, Timestamp};

use super::client::{RawAdditionalLimit, RawRateLimit, RawWindow, UsageResponse};
use super::labels::plan_label;
use super::number::FlexNumber;
use super::timestamp::from_epoch_seconds;
use super::windows::{Family, Slot, WindowReading, classify};

pub(super) fn map_usage(
    response: &UsageResponse,
    identity: AccountIdentity,
    now: Timestamp,
) -> LimitsSnapshot {
    let mut windows = response
        .rate_limit
        .as_ref()
        .map(|limit| classify(Family::Main, readings(limit, now)))
        .unwrap_or_default();
    for extra in response.additional_rate_limits.iter().flatten() {
        windows.extend(additional_windows(extra, now));
    }
    LimitsSnapshot {
        identity: with_plan(identity, response.plan_type.as_deref()),
        windows,
        balances: balances(response),
        notices: Vec::new(),
        fetched_at: now,
        source: LimitsSource::Live,
    }
}

pub(super) fn with_plan(mut identity: AccountIdentity, plan: Option<&str>) -> AccountIdentity {
    if let Some(plan) = plan.map(str::trim).filter(|plan| !plan.is_empty()) {
        identity.plan = Some(plan_label(plan));
    }
    identity
}

pub(super) fn credits_balance(value: u64) -> Balance {
    Balance {
        id: "credits".to_owned(),
        label: "Credits".to_owned(),
        amount: BalanceAmount::Count {
            value,
            unit: "credits".to_owned(),
        },
    }
}

pub(super) fn period_from_seconds(seconds: u64) -> Option<SignedDuration> {
    let seconds = i64::try_from(seconds).ok()?;
    (seconds > 0).then(|| SignedDuration::from_secs(seconds))
}

fn resets_balance(value: u64) -> Balance {
    Balance {
        id: "resets".to_owned(),
        label: "Resets".to_owned(),
        amount: BalanceAmount::Count {
            value,
            unit: "resets".to_owned(),
        },
    }
}

fn readings(limit: &RawRateLimit, now: Timestamp) -> Vec<WindowReading> {
    [
        (Slot::Primary, &limit.primary_window),
        (Slot::Secondary, &limit.secondary_window),
    ]
    .into_iter()
    .filter_map(|(slot, window)| reading(slot, window.as_ref()?, now))
    .collect()
}

fn reading(slot: Slot, window: &RawWindow, now: Timestamp) -> Option<WindowReading> {
    Some(WindowReading {
        slot,
        used: Percent::new(window.used_percent.as_ref()?.as_f64()?),
        period: window
            .limit_window_seconds
            .as_ref()
            .and_then(FlexNumber::whole)
            .and_then(period_from_seconds),
        resets_at: resets_at(window, now),
    })
}

fn resets_at(window: &RawWindow, now: Timestamp) -> Option<Timestamp> {
    let absolute = window
        .reset_at
        .as_ref()
        .and_then(FlexNumber::as_f64)
        .and_then(from_epoch_seconds);
    absolute.or_else(|| {
        let after = window.reset_after_seconds.as_ref()?.as_f64()?;
        now.checked_add(SignedDuration::try_from_secs_f64(after).ok()?)
            .ok()
    })
}

fn additional_windows(extra: &RawAdditionalLimit, now: Timestamp) -> Vec<QuotaWindow> {
    let name = [&extra.limit_name, &extra.metered_feature]
        .into_iter()
        .flatten()
        .map(|name| name.trim())
        .find(|name| !name.is_empty());
    match (name, &extra.rate_limit) {
        (Some(name), Some(limit)) => classify(Family::Model(name), readings(limit, now)),
        (None, Some(_)) => {
            tracing::warn!("skipping an additional codex rate limit without a name");
            Vec::new()
        }
        (_, None) => Vec::new(),
    }
}

fn balances(response: &UsageResponse) -> Vec<Balance> {
    let credits = response
        .credits
        .as_ref()
        .and_then(|credits| credits.balance.as_ref()?.whole())
        .map(credits_balance);
    let resets = response
        .rate_limit_reset_credits
        .as_ref()
        .and_then(|resets| resets.available_count.as_ref()?.whole())
        .map(resets_balance);
    credits.into_iter().chain(resets).collect()
}

#[cfg(test)]
#[path = "mapper_tests.rs"]
mod tests;
