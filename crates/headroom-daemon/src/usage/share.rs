use headroom_core::usage::UsageTotals;

pub const PERMILLE: i128 = 1_000;

/// Share of `whole` in permille, rounded down: by cost, or by tokens when `whole` cost nothing.
#[must_use]
pub fn share_permille(part: &UsageTotals, whole: &UsageTotals) -> u32 {
    let (part, whole) = if whole.cost.0 > 0 {
        (i128::from(part.cost.0), i128::from(whole.cost.0))
    } else {
        (
            i128::from(part.tokens.total().0),
            i128::from(whole.tokens.total().0),
        )
    };
    if whole <= 0 {
        return 0;
    }
    u32::try_from(part.max(0) * PERMILLE / whole).unwrap_or(u32::MAX)
}

#[cfg(test)]
mod tests {
    use headroom_core::tokens::TokenCounts;
    use headroom_core::units::{MicroUsd, Tokens};

    use super::*;

    fn totals(tokens: u64, cost: i64) -> UsageTotals {
        UsageTotals {
            tokens: TokenCounts {
                input: Tokens(tokens),
                ..TokenCounts::default()
            },
            cost: MicroUsd(cost),
            ..UsageTotals::default()
        }
    }

    #[test]
    fn shares_are_by_cost_rounded_down_or_by_tokens_when_free() {
        let table = [
            (totals(1, 733), totals(10, 1_000), 733),
            (totals(1, 1), totals(10, 3), 333),
            (totals(9, 2), totals(10, 3), 666),
            (totals(9, 0), totals(10, 3), 0),
            (totals(1, 0), totals(3, 0), 333),
            (totals(0, 0), totals(0, 0), 0),
            (totals(5, 5), totals(5, 5), 1_000),
        ];
        for (part, whole, expected) in table {
            assert_eq!(share_permille(&part, &whole), expected, "{part:?}");
        }
    }
}
