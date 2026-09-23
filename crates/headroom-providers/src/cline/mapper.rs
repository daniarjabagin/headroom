use headroom_core::pace::Tone;
use headroom_core::provider::ProviderError;
use headroom_core::quota::{Balance, BalanceAmount, Notice};
use headroom_core::units::MicroUsd;
use jiff::Timestamp;
use jiff::tz::TimeZone;

use super::raw::{RawBalance, RawCurrentPlan, RawOrganization, RawPlan};

const PAY_AS_YOU_GO: &str = "Pay as you go";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Fetched {
    pub(super) plan: Option<RawCurrentPlan>,
    pub(super) balance: RawBalance,
    pub(super) organization: Option<(RawOrganization, RawBalance)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Mapped {
    pub(super) plan: String,
    pub(super) balances: Vec<Balance>,
    pub(super) notices: Vec<Notice>,
}

pub(super) fn map_account(fetched: &Fetched) -> Result<Mapped, ProviderError> {
    let current = fetched.plan.as_ref();
    let plan_name = current
        .and_then(|current| current.plan.as_ref())
        .and_then(plan_name);
    let spendable = displayed_balance(fetched);
    let Some(plan_name) = plan_name else {
        return pay_as_you_go(fetched, spendable);
    };
    let mut notices: Vec<Notice> = current
        .and_then(|current| period_notice(&plan_name, current))
        .into_iter()
        .collect();
    if spendable <= 0 {
        notices.push(no_credits());
    }
    Ok(Mapped {
        plan: plan_name,
        balances: balances(fetched),
        notices,
    })
}

fn pay_as_you_go(fetched: &Fetched, spendable: i64) -> Result<Mapped, ProviderError> {
    if spendable <= 0 {
        return Err(ProviderError::NoSubscription {
            detail: "No Cline plan and no credits left.".to_owned(),
        });
    }
    Ok(Mapped {
        plan: PAY_AS_YOU_GO.to_owned(),
        balances: balances(fetched),
        notices: Vec::new(),
    })
}

fn plan_name(plan: &RawPlan) -> Option<String> {
    [&plan.display_name, &plan.name, &plan.id]
        .into_iter()
        .flatten()
        .map(|name| name.trim())
        .find(|name| !name.is_empty())
        .map(str::to_owned)
}

fn displayed_balance(fetched: &Fetched) -> i64 {
    match &fetched.organization {
        Some((_, balance)) => balance.balance,
        None => fetched.balance.balance,
    }
}

fn balances(fetched: &Fetched) -> Vec<Balance> {
    let personal = Balance {
        id: "credits".to_owned(),
        label: "Credits".to_owned(),
        amount: BalanceAmount::Usd(MicroUsd(fetched.balance.balance)),
    };
    let Some((organization, balance)) = &fetched.organization else {
        return vec![personal];
    };
    let name = organization
        .name
        .as_deref()
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .unwrap_or("Organization");
    let shared = Balance {
        id: "organization_credits".to_owned(),
        label: format!("{name} credits"),
        amount: BalanceAmount::Usd(MicroUsd(balance.balance)),
    };
    vec![shared, personal]
}

fn period_notice(plan: &str, current: &RawCurrentPlan) -> Option<Notice> {
    let (verb, at) = match current.cancel_at.as_deref().and_then(utc_date) {
        Some(date) => ("ends", date),
        None => (
            "renews",
            current.current_period_end.as_deref().and_then(utc_date)?,
        ),
    };
    Some(Notice {
        tone: Tone::Neutral,
        text: format!("{plan} {verb} on {at} (UTC)."),
    })
}

fn utc_date(text: &str) -> Option<String> {
    let at: Timestamp = text.parse().ok()?;
    Some(at.to_zoned(TimeZone::UTC).date().to_string())
}

fn no_credits() -> Notice {
    Notice {
        tone: Tone::Critical,
        text: "No Cline credits left.".to_owned(),
    }
}

#[cfg(test)]
#[path = "mapper_tests.rs"]
mod tests;
