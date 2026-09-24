use headroom_core::pace::Tone;
use headroom_core::provider::ProviderError;
use headroom_core::quota::{Balance, BalanceAmount, Notice};
use headroom_core::units::{CurrencyCode, Money};

use super::raw::RawBalance;
use super::region::Region;

pub(super) const USED_UP_NOTICE: &str = "Balance is used up; API requests fail until you top up";
pub(super) const DEBT_NOTICE: &str = "Cash balance is negative: the account is in debt";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Mapped {
    pub(super) balances: Vec<Balance>,
    pub(super) notices: Vec<Notice>,
}

pub(super) fn map(region: Region, raw: &RawBalance) -> Result<Mapped, ProviderError> {
    let currency = CurrencyCode::parse(region.currency())
        .map_err(|error| ProviderError::InvalidResponse(error.to_string()))?;
    let amount = |micros: i64| {
        BalanceAmount::Money(Money {
            currency: currency.clone(),
            micros,
        })
    };
    let balances = [
        ("available", "Balance", raw.available.0),
        ("voucher", "Vouchers", raw.voucher.0),
        ("cash", "Cash", raw.cash.0),
    ]
    .into_iter()
    .map(|(id, label, micros)| Balance {
        id: id.to_owned(),
        label: label.to_owned(),
        amount: amount(micros),
    })
    .collect();
    Ok(Mapped {
        balances,
        notices: notices(raw),
    })
}

fn notices(raw: &RawBalance) -> Vec<Notice> {
    [
        (raw.available.0 <= 0, USED_UP_NOTICE),
        (raw.cash.0 < 0, DEBT_NOTICE),
    ]
    .into_iter()
    .filter(|(applies, _)| *applies)
    .map(|(_, text)| Notice {
        tone: Tone::Critical,
        text: text.to_owned(),
    })
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decimal::ExactMicros;

    const BALANCE: &str = include_str!("fixtures/balance.json");
    const IN_DEBT: &str = include_str!("fixtures/balance_in_debt.json");

    fn raw(body: &str) -> RawBalance {
        let envelope: super::super::raw::RawEnvelope = serde_json::from_str(body).unwrap();
        envelope.data.unwrap()
    }

    fn money(id: &str, label: &str, currency: &str, micros: i64) -> Balance {
        Balance {
            id: id.into(),
            label: label.into(),
            amount: BalanceAmount::Money(Money {
                currency: CurrencyCode::parse(currency).unwrap(),
                micros,
            }),
        }
    }

    #[test]
    fn global_keys_are_in_dollars() {
        let mapped = map(Region::Global, &raw(BALANCE)).unwrap();
        assert_eq!(
            mapped.balances,
            [
                money("available", "Balance", "USD", 49_588_940),
                money("voucher", "Vouchers", "USD", 46_588_930),
                money("cash", "Cash", "USD", 3_000_010),
            ]
        );
        assert_eq!(mapped.notices, []);
    }

    #[test]
    fn mainland_keys_are_in_yuan() {
        let mapped = map(Region::Mainland, &raw(BALANCE)).unwrap();
        assert_eq!(
            mapped.balances[0],
            money("available", "Balance", "CNY", 49_588_940)
        );
    }

    #[test]
    fn debt_is_shown_negative_with_critical_notices() {
        let mapped = map(Region::Mainland, &raw(IN_DEBT)).unwrap();
        assert_eq!(
            mapped.balances[0],
            money("available", "Balance", "CNY", -1_234_568)
        );
        assert_eq!(mapped.balances[1], money("voucher", "Vouchers", "CNY", 0));
        let texts: Vec<_> = mapped.notices.iter().map(|n| n.text.as_str()).collect();
        assert_eq!(texts, [USED_UP_NOTICE, DEBT_NOTICE]);
        assert!(mapped.notices.iter().all(|n| n.tone == Tone::Critical));
    }

    #[test]
    fn a_zero_balance_is_used_up_without_debt() {
        let empty = RawBalance {
            available: ExactMicros(0),
            voucher: ExactMicros(0),
            cash: ExactMicros(0),
        };
        let mapped = map(Region::Global, &empty).unwrap();
        assert_eq!(
            mapped.notices,
            [Notice {
                tone: Tone::Critical,
                text: USED_UP_NOTICE.into(),
            }]
        );
    }
}
