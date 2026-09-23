use headroom_core::units::MicroUsd;

const MICRO_DIGITS: usize = 6;
const MICROS_PER_USD: i64 = 1_000_000;

pub(super) fn micro_usd(text: &str) -> Option<MicroUsd> {
    let text = text.trim();
    let (negative, unsigned) = match text.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, text),
    };
    let (whole, fraction) = unsigned.split_once('.').unwrap_or((unsigned, ""));
    if whole.is_empty() && fraction.is_empty() || !all_digits(whole) || !all_digits(fraction) {
        return None;
    }
    let dollars = if whole.is_empty() {
        0
    } else {
        whole.parse::<i64>().ok()?
    };
    let micros = dollars
        .checked_mul(MICROS_PER_USD)?
        .checked_add(fraction_micros(fraction)?)?;
    Some(MicroUsd(if negative { -micros } else { micros }))
}

fn all_digits(text: &str) -> bool {
    text.bytes().all(|byte| byte.is_ascii_digit())
}

fn fraction_micros(fraction: &str) -> Option<i64> {
    let kept = fraction.get(..MICRO_DIGITS.min(fraction.len()))?;
    let padded = format!("{kept:0<MICRO_DIGITS$}");
    let micros = padded.parse::<i64>().ok()?;
    let rounds_up = fraction
        .as_bytes()
        .get(MICRO_DIGITS)
        .is_some_and(|digit| *digit >= b'5');
    Some(micros + i64::from(rounds_up))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decimal_strings_convert_exactly() {
        let cases = [
            ("0.00000", 0),
            ("0", 0),
            ("12.5", 12_500_000),
            ("3.141592", 3_141_592),
            (".25", 250_000),
            ("7.", 7_000_000),
            ("-1.5", -1_500_000),
            (" 42.000001 ", 42_000_001),
        ];
        for (text, micros) in cases {
            assert_eq!(micro_usd(text), Some(MicroUsd(micros)), "{text}");
        }
    }

    #[test]
    fn sub_micro_digits_round_half_up_to_the_nearest_micro() {
        assert_eq!(micro_usd("0.0000004"), Some(MicroUsd(0)));
        assert_eq!(micro_usd("0.0000005"), Some(MicroUsd(1)));
        assert_eq!(micro_usd("1.9999999"), Some(MicroUsd(2_000_000)));
    }

    #[test]
    fn non_decimal_text_is_rejected() {
        for text in [
            "",
            ".",
            "-",
            "1e-5",
            "abc",
            "1.2.3",
            "$1",
            "1,5",
            "99999999999999999",
        ] {
            assert_eq!(micro_usd(text), None, "{text}");
        }
    }
}
