use headroom_core::pace::Tone;
use headroom_core::provider::ProviderError;
use headroom_core::quota::{Balance, BalanceAmount, Notice, QuotaWindow, WindowId};
use headroom_core::units::{MicroUsd, Percent};
use jiff::civil::Date;
use jiff::tz::TimeZone;
use jiff::{SignedDuration, Span, Timestamp};
use serde_json::{Number, Value};

use super::credits::credits_to_micro_usd;
use super::raw::{RawCounts, RawSnapshot, RawUser};

const NO_PLAN: &str = "No GitHub Copilot plan on this account.";
const ORG_SEAT_NOTICE: &str = "This seat is managed by an organization: GitHub reports its \
                               usage only to the organization's billing managers.";

#[derive(Debug, Clone, PartialEq)]
pub(super) struct MappedUser {
    pub(super) plan: Option<String>,
    pub(super) windows: Vec<QuotaWindow>,
    pub(super) balances: Vec<Balance>,
    pub(super) notices: Vec<Notice>,
}

#[derive(Clone, Copy)]
struct Cycle {
    resets_at: Option<Timestamp>,
    period: Option<SignedDuration>,
}

pub(super) fn no_subscription() -> ProviderError {
    ProviderError::NoSubscription {
        detail: NO_PLAN.to_owned(),
    }
}

pub(super) fn map_user(raw: &RawUser) -> Result<MappedUser, ProviderError> {
    let cycle = cycle(raw)?;
    let snapshots = raw.quota_snapshots.clone().unwrap_or_default();
    let premium = snapshots
        .premium_interactions
        .or(snapshots.premium_requests);
    let mut mapped = MappedUser {
        plan: raw.copilot_plan.as_deref().and_then(plan_label),
        windows: Vec::new(),
        balances: Vec::new(),
        notices: Vec::new(),
    };
    if let Some(credits) = premium
        .as_ref()
        .and_then(|s| window("credits", "Credits", s, cycle))
    {
        mapped.windows.push(credits);
        mapped
            .balances
            .extend(premium.as_ref().map(extra_usage).transpose()?.flatten());
    }
    let chat = snapshots
        .chat
        .as_ref()
        .and_then(|s| window("chat", "Chat", s, cycle));
    let completions = snapshots.completions.as_ref();
    let completions = completions.and_then(|s| window("completions", "Completions", s, cycle));
    mapped.windows.extend(chat.into_iter().chain(completions));
    if mapped.windows.is_empty() {
        mapped.windows = legacy_windows(raw, cycle);
    }
    if mapped.windows.is_empty() {
        return org_seat(mapped, raw, premium.as_ref());
    }
    Ok(mapped)
}

fn org_seat(
    mut mapped: MappedUser,
    raw: &RawUser,
    premium: Option<&RawSnapshot>,
) -> Result<MappedUser, ProviderError> {
    if raw.token_based_billing != Some(Value::Bool(true)) {
        return Err(no_subscription());
    }
    let used = premium.and_then(|s| s.credits_used.as_ref());
    if let Some(amount) = used.map(credits).transpose()?.filter(|a| a.0 > 0) {
        mapped.balances.push(Balance {
            id: "credits_used".to_owned(),
            label: "Credits used".to_owned(),
            amount: BalanceAmount::Usd(amount),
        });
    }
    mapped.notices.push(Notice {
        tone: Tone::Neutral,
        text: ORG_SEAT_NOTICE.to_owned(),
    });
    Ok(mapped)
}

fn window(id: &str, label: &str, raw: &RawSnapshot, cycle: Cycle) -> Option<QuotaWindow> {
    let entitlement = raw.entitlement.as_ref().and_then(Number::as_f64);
    let remaining = raw.remaining.as_ref().and_then(Number::as_f64);
    let no_allotment = entitlement.is_some_and(|total| total <= 0.0);
    if raw.unlimited == Some(true) || no_allotment || remaining.is_some_and(|left| left < 0.0) {
        return None;
    }
    let used = match (raw.percent_remaining.as_ref(), entitlement, remaining) {
        (Some(percent), _, _) => 100.0 - percent.as_f64()?,
        (None, Some(total), Some(left)) => 100.0 - left / total * 100.0,
        _ => return None,
    };
    Some(quota_window(id, label, used, cycle))
}

fn legacy_windows(raw: &RawUser, cycle: Cycle) -> Vec<QuotaWindow> {
    let left = raw.limited_user_quotas.clone().unwrap_or_default();
    let total = raw.monthly_quotas.clone().unwrap_or_default();
    let pick = |counts: &RawCounts, chat: bool| {
        let value = if chat {
            &counts.chat
        } else {
            &counts.completions
        };
        value.as_ref().and_then(Number::as_f64)
    };
    [
        ("chat", "Chat", true),
        ("completions", "Completions", false),
    ]
    .into_iter()
    .filter_map(|(id, label, chat)| {
        let total = pick(&total, chat).filter(|total| *total > 0.0)?;
        let left = pick(&left, chat)?;
        Some(quota_window(
            id,
            label,
            (total - left) / total * 100.0,
            cycle,
        ))
    })
    .collect()
}

fn quota_window(id: &str, label: &str, used: f64, cycle: Cycle) -> QuotaWindow {
    QuotaWindow {
        id: WindowId::Other(id.to_owned()),
        label: label.to_owned(),
        used: Percent::new(used),
        resets_at: cycle.resets_at,
        period: cycle.period,
    }
}

fn extra_usage(raw: &RawSnapshot) -> Result<Option<Balance>, ProviderError> {
    if raw.overage_permitted != Some(true) {
        return Ok(None);
    }
    let amount = match raw.overage_count.as_ref() {
        Some(count) => credits(count)?,
        None => MicroUsd::ZERO,
    };
    Ok(Some(Balance {
        id: "extra_usage".to_owned(),
        label: "Extra usage".to_owned(),
        amount: BalanceAmount::Usd(amount),
    }))
}

fn credits(number: &Number) -> Result<MicroUsd, ProviderError> {
    credits_to_micro_usd(number).ok_or_else(|| {
        ProviderError::InvalidResponse(format!("Copilot reported unreadable credits {number}"))
    })
}

fn cycle(raw: &RawUser) -> Result<Cycle, ProviderError> {
    let reset = [&raw.quota_reset_date, &raw.limited_user_reset_date]
        .into_iter()
        .filter_map(|value| value.as_deref().map(str::trim))
        .find(|value| !value.is_empty());
    let Some(text) = reset else {
        return Ok(Cycle {
            resets_at: None,
            period: None,
        });
    };
    let resets_at = reset_time(text).ok_or_else(|| {
        ProviderError::InvalidResponse(format!("Copilot reported an unreadable reset date {text}"))
    })?;
    Ok(Cycle {
        resets_at: Some(resets_at),
        period: month_before(resets_at),
    })
}

fn reset_time(text: &str) -> Option<Timestamp> {
    text.parse::<Timestamp>().ok().or_else(|| {
        let date = text.parse::<Date>().ok()?;
        date.to_zoned(TimeZone::UTC)
            .ok()
            .map(|zoned| zoned.timestamp())
    })
}

fn month_before(resets_at: Timestamp) -> Option<SignedDuration> {
    let start = resets_at
        .to_zoned(TimeZone::UTC)
        .checked_sub(Span::new().months(1))
        .ok()?;
    Some(resets_at.duration_since(start.timestamp()))
}

fn plan_label(raw: &str) -> Option<String> {
    let words: Vec<String> = raw
        .split(['_', '-', ' '])
        .filter(|word| !word.is_empty())
        .map(title_case)
        .collect();
    (!words.is_empty()).then(|| words.join(" "))
}

fn title_case(word: &str) -> String {
    let mut chars = word.chars();
    chars.next().map_or_else(String::new, |first| {
        first
            .to_uppercase()
            .chain(chars.flat_map(char::to_lowercase))
            .collect()
    })
}

#[cfg(test)]
#[path = "mapper_tests.rs"]
mod tests;
