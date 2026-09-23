use headroom_core::units::MicroUsd;
use serde_json::Number;

const MICRO_USD_PER_CREDIT: i64 = 10_000;
const CREDIT_FRACTION_DIGITS: usize = 4;

/// GitHub AI credits (1 credit = USD 0.01) as micro-USD, read from the number's decimal text.
pub(super) fn credits_to_micro_usd(credits: &Number) -> Option<MicroUsd> {
    let text = credits.to_string();
    let (negative, digits) = match text.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, text.as_str()),
    };
    let (whole, fraction) = digits.split_once('.').unwrap_or((digits, ""));
    let magnitude = scaled(whole, fraction)?;
    Some(MicroUsd(if negative { -magnitude } else { magnitude }))
}

fn scaled(whole: &str, fraction: &str) -> Option<i64> {
    if !all_digits(whole) || !(fraction.is_empty() || all_digits(fraction)) {
        return None;
    }
    let kept = fraction.get(..CREDIT_FRACTION_DIGITS).unwrap_or(fraction);
    let padded = format!("{kept:0<CREDIT_FRACTION_DIGITS$}");
    let round_up = fraction
        .as_bytes()
        .get(CREDIT_FRACTION_DIGITS)
        .is_some_and(|digit| *digit >= b'5');
    whole
        .parse::<i64>()
        .ok()?
        .checked_mul(MICRO_USD_PER_CREDIT)?
        .checked_add(padded.parse::<i64>().ok()?)?
        .checked_add(i64::from(round_up))
}

fn all_digits(text: &str) -> bool {
    !text.is_empty() && text.bytes().all(|b| b.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn micro(text: &str) -> Option<i64> {
        let number: Number = serde_json::from_str(text).unwrap();
        credits_to_micro_usd(&number).map(|amount| amount.0)
    }

    #[test]
    fn whole_and_fractional_credits_convert_exactly() {
        let cases = [
            ("0", Some(0)),
            ("1", Some(10_000)),
            ("2111", Some(21_110_000)),
            ("0.5", Some(5_000)),
            ("12.3456", Some(123_456)),
            ("-3.25", Some(-32_500)),
            ("0.0001", Some(1)),
        ];
        for (text, expected) in cases {
            assert_eq!(micro(text), expected, "{text}");
        }
    }

    #[test]
    fn digits_below_a_micro_dollar_round_half_up() {
        assert_eq!(micro("298.698546"), Some(2_986_985));
        assert_eq!(micro("0.00004"), Some(0));
        assert_eq!(micro("0.00005"), Some(1));
    }

    #[test]
    fn exponents_and_overflow_are_rejected() {
        assert_eq!(micro("1e20"), None);
        assert_eq!(micro("18446744073709551615"), None);
    }
}
