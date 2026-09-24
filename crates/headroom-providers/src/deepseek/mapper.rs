use std::collections::BTreeSet;

use headroom_core::pace::Tone;
use headroom_core::provider::ProviderError;
use headroom_core::quota::{Balance, BalanceAmount, Notice};
use headroom_core::units::{CurrencyCode, Money};

use super::raw::{RawBalance, RawBalanceInfo};

pub(super) const UNAVAILABLE_NOTICE: &str = "Balance is not enough for API calls";
pub(super) const USED_UP_NOTICE: &str = "Balance is used up; API calls fail until you top up";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Mapped {
    pub(super) balances: Vec<Balance>,
    pub(super) notices: Vec<Notice>,
}

pub(super) fn map(raw: &RawBalance) -> Result<Mapped, ProviderError> {
    let balances = raw
        .balance_infos
        .iter()
        .map(balance)
        .collect::<Result<Vec<_>, _>>()?;
    reject_repeated_currencies(&balances)?;
    let notices = (!raw.is_available)
        .then(|| unavailable_notice(&balances))
        .into_iter()
        .collect();
    Ok(Mapped { balances, notices })
}

fn balance(info: &RawBalanceInfo) -> Result<Balance, ProviderError> {
    let currency = CurrencyCode::parse(&info.currency)
        .map_err(|error| ProviderError::InvalidResponse(format!("DeepSeek balance: {error}")))?;
    Ok(Balance {
        id: format!("balance_{}", currency.as_str().to_ascii_lowercase()),
        label: "Balance".to_owned(),
        amount: BalanceAmount::Money(Money {
            currency,
            micros: info.total_balance.0,
        }),
    })
}

fn reject_repeated_currencies(balances: &[Balance]) -> Result<(), ProviderError> {
    let mut seen = BTreeSet::new();
    match balances.iter().find(|b| !seen.insert(b.id.as_str())) {
        Some(repeated) => Err(ProviderError::InvalidResponse(format!(
            "DeepSeek listed the {} balance twice",
            repeated.id
        ))),
        None => Ok(()),
    }
}

fn unavailable_notice(balances: &[Balance]) -> Notice {
    let used_up = balances.iter().all(|b| match &b.amount {
        BalanceAmount::Money(money) => money.micros <= 0,
        BalanceAmount::Usd(_) | BalanceAmount::Count { .. } => false,
    });
    if used_up {
        Notice {
            tone: Tone::Critical,
            text: USED_UP_NOTICE.to_owned(),
        }
    } else {
        Notice {
            tone: Tone::Warning,
            text: UNAVAILABLE_NOTICE.to_owned(),
        }
    }
}

#[cfg(test)]
#[path = "mapper_tests.rs"]
mod tests;
