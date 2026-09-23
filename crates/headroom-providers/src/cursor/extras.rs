use headroom_core::quota::{Balance, QuotaWindow};
use headroom_core::units::{MicroUsd, Percent};
use jiff::tz::TimeZone;
use jiff::{Timestamp, ToSpan};

use super::mapper::{Cycle, cycle_between, decimal, usd_balance, window};
use super::number::Decimal;
use super::raw::{RawCreditGrants, RawGrokBotUsage, RawRequestUsage, RawStripe};

pub(super) fn credits(
    grants: Option<&RawCreditGrants>,
    stripe: Option<&RawStripe>,
) -> Option<Balance> {
    let (granted, used) = grants
        .and_then(valid_grants)
        .unwrap_or((MicroUsd::ZERO, MicroUsd::ZERO));
    let prepaid = stripe.and_then(prepaid_balance).unwrap_or(MicroUsd::ZERO);
    let total = granted.0.checked_add(prepaid.0)?;
    if total <= 0 {
        return None;
    }
    let left = total.saturating_sub(used.0).max(0);
    Some(usd_balance("credits", "Credits", MicroUsd(left)))
}

fn valid_grants(grants: &RawCreditGrants) -> Option<(MicroUsd, MicroUsd)> {
    if grants.has_credit_grants != Some(true) {
        return None;
    }
    let cents = |value| decimal(value).and_then(Decimal::cents_to_micro_usd);
    let total = cents(grants.total_cents.as_ref()).filter(|total| total.0 > 0);
    let used = cents(grants.used_cents.as_ref()).filter(|used| used.0 >= 0);
    let valid = total.zip(used);
    if valid.is_none() {
        tracing::warn!("ignoring Cursor credit grants with invalid amounts");
    }
    valid
}

fn prepaid_balance(stripe: &RawStripe) -> Option<MicroUsd> {
    let balance = decimal(stripe.customer_balance.as_ref())?;
    if !balance.is_negative() {
        return None;
    }
    balance
        .cents_to_micro_usd()
        .and_then(|amount| amount.0.checked_neg())
        .map(MicroUsd)
}

pub(super) fn grok_bot_window(raw: &RawGrokBotUsage) -> Option<QuotaWindow> {
    let no_personal_allowance = raw.uses_pooled_enterprise_allowance == Some(true)
        || raw.has_non_zero_included_limit == Some(false)
        || raw.included_limit_zero == Some(true);
    if no_personal_allowance {
        return None;
    }
    let used = decimal(raw.usage_percent.as_ref())
        .filter(|used| !used.is_negative())?
        .to_f64();
    let time = |text: &Option<String>| {
        text.as_deref()
            .and_then(|t| t.trim().parse::<Timestamp>().ok())
    };
    let cycle = cycle_between(
        time(&raw.current_period_start),
        time(&raw.next_reset_timestamp_utc),
    );
    Some(window("grok_bot", "Grok Bot", Percent::new(used), cycle))
}

pub(super) fn request_window(raw: &RawRequestUsage) -> Option<QuotaWindow> {
    let model = raw.gpt4.as_ref()?;
    let limit = decimal(model.max_request_usage.as_ref())
        .filter(|limit| limit.is_positive())?
        .to_f64();
    let used = decimal(model.num_requests.as_ref())
        .or_else(|| decimal(model.num_requests_total.as_ref()))?
        .to_f64();
    let start = raw
        .start_of_month
        .as_deref()
        .and_then(|text| text.trim().parse::<Timestamp>().ok());
    let used = Percent::new(used / limit * 100.0);
    Some(window("requests", "Requests", used, monthly_cycle(start)))
}

fn monthly_cycle(start: Option<Timestamp>) -> Cycle {
    let end = start.and_then(|start| {
        start
            .to_zoned(TimeZone::UTC)
            .checked_add(1.month())
            .ok()
            .map(|end| end.timestamp())
    });
    cycle_between(start, end)
}

#[cfg(test)]
#[path = "extras_tests.rs"]
mod tests;
