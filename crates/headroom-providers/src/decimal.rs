use serde::de::Error as _;
use serde::{Deserialize, Deserializer};
use serde_json::value::RawValue;

const MICROS_PER_UNIT: i128 = 1_000_000;
const MICRO_DIGITS: usize = 6;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ExactMicros(pub(crate) i64);

impl<'de> Deserialize<'de> for ExactMicros {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<ExactMicros, D::Error> {
        let raw = Box::<RawValue>::deserialize(deserializer)?;
        let text = raw.get();
        let digits = if text.starts_with('"') {
            serde_json::from_str::<String>(text).map_err(D::Error::custom)?
        } else {
            text.to_owned()
        };
        parse_micros(&digits)
            .map(ExactMicros)
            .ok_or_else(|| D::Error::custom(format!("{text} is not an exact decimal amount")))
    }
}

pub(crate) fn parse_micros(text: &str) -> Option<i64> {
    let text = text.trim();
    let (negative, unsigned) = match text.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, text),
    };
    let (whole, fraction) = unsigned.split_once('.').unwrap_or((unsigned, ""));
    let well_formed = !(whole.is_empty() && fraction.is_empty())
        && whole
            .bytes()
            .chain(fraction.bytes())
            .all(|b| b.is_ascii_digit());
    if !well_formed {
        return None;
    }
    let magnitude = unsigned_micros(whole, fraction)?;
    let signed = if negative { -magnitude } else { magnitude };
    i64::try_from(signed).ok()
}

fn unsigned_micros(whole: &str, fraction: &str) -> Option<i128> {
    let kept = fraction.get(..MICRO_DIGITS).unwrap_or(fraction);
    let dropped = fraction.get(MICRO_DIGITS..).unwrap_or("");
    let micros = digits_value(whole)?
        .checked_mul(MICROS_PER_UNIT)?
        .checked_add(digits_value(kept)?.checked_mul(scale(kept.len()))?)?;
    let round_up = dropped.bytes().next().is_some_and(|digit| digit >= b'5');
    micros.checked_add(i128::from(round_up))
}

fn digits_value(digits: &str) -> Option<i128> {
    digits.bytes().try_fold(0_i128, |value, digit| {
        value.checked_mul(10)?.checked_add(i128::from(digit - b'0'))
    })
}

fn scale(kept_digits: usize) -> i128 {
    (kept_digits..MICRO_DIGITS).fold(1, |value, _| value * 10)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn exact(json: &str) -> Result<ExactMicros, serde_json::Error> {
        serde_json::from_str(json)
    }

    #[test]
    fn decimal_text_converts_exactly() {
        let cases = [
            ("0", 0),
            ("12", 12_000_000),
            ("6.5", 6_500_000),
            ("0.000001", 1),
            ("0.0000004", 0),
            ("0.0000005", 1),
            ("-0.0000005", -1),
            ("123.456789", 123_456_789),
            ("0.1234564999", 123_456),
            ("49.58894", 49_588_940),
            ("-2.25", -2_250_000),
            ("-0.00", 0),
            (".5", 500_000),
            ("7.", 7_000_000),
            ("0000110.00", 110_000_000),
            ("1.00000000000000000000000000000000000001", 1_000_000),
            ("9223372036854.775807", i64::MAX),
            ("-9223372036854.775808", i64::MIN),
        ];
        for (text, micros) in cases {
            assert_eq!(parse_micros(text), Some(micros), "{text}");
        }
    }

    #[test]
    fn malformed_scientific_or_overflowing_text_is_rejected() {
        for text in [
            "",
            ".",
            "-",
            "--1",
            "1e-5",
            "4.958894E1",
            "1,5",
            "abc",
            "1.2.3",
            "+1",
            "NaN",
            "Infinity",
            "9223372036855",
            "99999999999999999999999999999999999999999",
        ] {
            assert_eq!(parse_micros(text), None, "{text}");
        }
    }

    #[test]
    fn json_strings_and_numbers_keep_every_written_decimal() {
        let cases = [
            ("\"110.00\"", 110_000_000),
            ("\"-3.50\"", -3_500_000),
            ("49.58894", 49_588_940),
            ("0.1", 100_000),
            ("0.30000000000000004", 300_000),
            ("-3.00001", -3_000_010),
            ("12", 12_000_000),
        ];
        for (json, micros) in cases {
            assert_eq!(exact(json).unwrap(), ExactMicros(micros), "{json}");
        }
    }

    #[test]
    fn json_values_that_are_not_exact_decimals_are_errors() {
        for json in [
            "1e2",
            "4.958894E1",
            "\"1e2\"",
            "\"\"",
            "\"lots\"",
            "true",
            "null",
            "[1]",
            "18446744073709551615",
        ] {
            assert!(exact(json).is_err(), "{json}");
        }
    }
}
