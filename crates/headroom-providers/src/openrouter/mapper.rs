use headroom_core::pace::Tone;
use headroom_core::quota::{Balance, BalanceAmount, Notice, QuotaWindow, WindowId};
use headroom_core::units::{MicroUsd, Percent};

use super::client::Credits;
use super::money::Usd;
use super::raw::{RawCredits, RawKey};

pub(super) const MANAGEMENT_KEY_NOTICE: &str = "Credit balance needs a management key";
pub(super) const CREDITS_UNAVAILABLE_NOTICE: &str = "Credit balance is unavailable right now";
const KEY_LIMIT_ID: &str = "key_limit";

#[derive(Debug, Clone, PartialEq)]
pub(super) struct Mapped {
    pub(super) plan: Option<String>,
    pub(super) windows: Vec<QuotaWindow>,
    pub(super) balances: Vec<Balance>,
    pub(super) notices: Vec<Notice>,
}

pub(super) fn map(key: &RawKey, credits: &Credits) -> Mapped {
    let mut balances = Vec::new();
    let mut notices = Vec::new();
    match credits {
        Credits::Available(credits) => balances.push(credit_balance(credits)),
        Credits::NeedsManagementKey => notices.push(neutral(MANAGEMENT_KEY_NOTICE)),
        Credits::Unavailable(error) => {
            tracing::warn!(%error, "OpenRouter credit balance unavailable");
            notices.push(neutral(CREDITS_UNAVAILABLE_NOTICE));
        }
    }
    balances.extend(spend_balances(key));
    Mapped {
        plan: plan(key),
        windows: key_limit_window(key).into_iter().collect(),
        balances,
        notices,
    }
}

pub(super) fn plan(key: &RawKey) -> Option<String> {
    key.is_free_tier
        .map(|free| if free { "Free tier" } else { "Pay as you go" }.to_owned())
}

fn credit_balance(credits: &RawCredits) -> Balance {
    let left = credits
        .total_credits
        .0
        .0
        .saturating_sub(credits.total_usage.0.0);
    usd_balance("credits", "Credit balance", MicroUsd(left))
}

fn spend_balances(key: &RawKey) -> impl Iterator<Item = Balance> {
    [
        ("spend_today", "Spent today", key.usage_daily),
        ("spend_week", "Spent this week", key.usage_weekly),
        ("spend_month", "Spent this month", key.usage_monthly),
    ]
    .into_iter()
    .filter_map(|(id, label, amount)| amount.map(|Usd(amount)| usd_balance(id, label, amount)))
}

fn key_limit_window(key: &RawKey) -> Option<QuotaWindow> {
    let Usd(limit) = key.limit?;
    if limit.0 <= 0 {
        return None;
    }
    let used = match (key.limit_remaining, key.limit_reset.as_deref()) {
        (Some(Usd(remaining)), _) => limit.0.saturating_sub(remaining.0).max(0),
        (None, None) => key.usage?.0.0,
        (None, Some(_)) => return None,
    };
    Some(QuotaWindow {
        id: WindowId::Other(KEY_LIMIT_ID.to_owned()),
        label: "Key limit".to_owned(),
        used: share_of(used, limit.0),
        resets_at: None,
        period: None,
    })
}

#[allow(
    clippy::cast_precision_loss,
    reason = "a display percentage of two exact micro-USD amounts"
)]
fn share_of(used: i64, limit: i64) -> Percent {
    Percent::new(used as f64 * 100.0 / limit as f64)
}

fn usd_balance(id: &str, label: &str, amount: MicroUsd) -> Balance {
    Balance {
        id: id.to_owned(),
        label: label.to_owned(),
        amount: BalanceAmount::Usd(amount),
    }
}

fn neutral(text: &str) -> Notice {
    Notice {
        tone: Tone::Neutral,
        text: text.to_owned(),
    }
}

#[cfg(test)]
#[path = "mapper_tests.rs"]
mod tests;
