use headroom_core::quota::{Balance, BalanceAmount};

use super::client::RawBalance;

pub(super) const POINTS_ID: &str = "points";

pub(super) fn point_balance(raw: &RawBalance) -> Balance {
    Balance {
        id: POINTS_ID.to_owned(),
        label: "Point balance".to_owned(),
        amount: BalanceAmount::Count {
            value: raw.current_point_balance,
            unit: "points".to_owned(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_balance_is_an_exact_point_count() {
        let raw: RawBalance =
            serde_json::from_str(include_str!("fixtures/current_balance.json")).unwrap();
        assert_eq!(
            point_balance(&raw),
            Balance {
                id: "points".into(),
                label: "Point balance".into(),
                amount: BalanceAmount::Count {
                    value: 842_150,
                    unit: "points".into()
                },
            }
        );
    }
}
