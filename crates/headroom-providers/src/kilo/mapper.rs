use headroom_core::pace::Tone;
use headroom_core::quota::{Balance, BalanceAmount, Notice};

use super::client::RawBalance;
use super::money::Usd;

pub(super) const CREDITS_ID: &str = "credits";
pub(super) const DEPLETED_NOTICE: &str = "Kilo credits are used up";

#[derive(Debug, Clone, PartialEq)]
pub(super) struct Mapped {
    pub(super) balances: Vec<Balance>,
    pub(super) notices: Vec<Notice>,
}

pub(super) fn map(raw: &RawBalance, organization: bool) -> Mapped {
    let Usd(amount) = raw.balance;
    let label = if organization {
        "Organization credits"
    } else {
        "Credit balance"
    };
    let notices = if raw.is_depleted == Some(true) {
        vec![Notice {
            tone: Tone::Critical,
            text: DEPLETED_NOTICE.to_owned(),
        }]
    } else {
        Vec::new()
    };
    Mapped {
        balances: vec![Balance {
            id: CREDITS_ID.to_owned(),
            label: label.to_owned(),
            amount: BalanceAmount::Usd(amount),
        }],
        notices,
    }
}

#[cfg(test)]
mod tests {
    use headroom_core::units::MicroUsd;

    use super::*;

    fn raw(body: &str) -> RawBalance {
        serde_json::from_str(body).unwrap()
    }

    #[test]
    fn a_personal_balance_is_exact_usd_without_notices() {
        let mapped = map(&raw(include_str!("fixtures/balance.json")), false);
        assert_eq!(
            mapped.balances,
            [Balance {
                id: "credits".into(),
                label: "Credit balance".into(),
                amount: BalanceAmount::Usd(MicroUsd(18_123_456)),
            }]
        );
        assert_eq!(mapped.notices, []);
    }

    #[test]
    fn a_depleted_organization_balance_is_critical_and_keeps_its_sign() {
        let mapped = map(&raw(include_str!("fixtures/balance_depleted.json")), true);
        assert_eq!(mapped.balances[0].label, "Organization credits");
        assert_eq!(
            mapped.balances[0].amount,
            BalanceAmount::Usd(MicroUsd(-421_300))
        );
        assert_eq!(
            mapped.notices,
            [Notice {
                tone: Tone::Critical,
                text: DEPLETED_NOTICE.into()
            }]
        );
    }

    #[test]
    fn a_missing_depleted_flag_adds_no_notice() {
        let mapped = map(&raw(r#"{"balance":0}"#), false);
        assert_eq!(mapped.notices, []);
        assert_eq!(mapped.balances[0].amount, BalanceAmount::Usd(MicroUsd(0)));
    }
}
