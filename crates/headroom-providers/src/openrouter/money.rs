use headroom_core::units::MicroUsd;
use serde::Deserialize;
use serde_json::Number;

const MICROS_PER_USD: i128 = 1_000_000;
const MICRO_DIGITS: usize = 6;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(try_from = "RawAmount")]
pub(super) struct Usd(pub(super) MicroUsd);

#[derive(Deserialize)]
#[serde(untagged)]
enum RawAmount {
    Number(Number),
    Text(String),
}

impl TryFrom<RawAmount> for Usd {
    type Error = String;

    fn try_from(raw: RawAmount) -> Result<Usd, String> {
        let parsed = match &raw {
            RawAmount::Number(number) => number_to_micros(number),
            RawAmount::Text(text) => parse_usd(text),
        };
        parsed.map(Usd).ok_or_else(|| match raw {
            RawAmount::Number(number) => format!("{number} is not a USD amount"),
            RawAmount::Text(text) => format!("{text:?} is not a USD amount"),
        })
    }
}

fn number_to_micros(number: &Number) -> Option<MicroUsd> {
    if let Some(whole) = number.as_i64() {
        return whole.checked_mul(1_000_000).map(MicroUsd);
    }
    if number.is_u64() {
        return None;
    }
    number
        .as_f64()
        .and_then(|value| parse_usd(&value.to_string()))
}

/// Exact decimal text to micro-USD, rounding the seventh decimal half away from zero.
pub(super) fn parse_usd(text: &str) -> Option<MicroUsd> {
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
    i64::try_from(signed).ok().map(MicroUsd)
}

fn unsigned_micros(whole: &str, fraction: &str) -> Option<i128> {
    let kept = fraction.get(..MICRO_DIGITS).unwrap_or(fraction);
    let dropped = fraction.get(MICRO_DIGITS..).unwrap_or("");
    let micros = digits_value(whole)?
        .checked_mul(MICROS_PER_USD)?
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

    fn usd(json: &str) -> Result<Usd, serde_json::Error> {
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
            ("-2.25", -2_250_000),
            (".5", 500_000),
            ("7.", 7_000_000),
            ("9223372036854.775807", i64::MAX),
        ];
        for (text, micros) in cases {
            assert_eq!(parse_usd(text), Some(MicroUsd(micros)), "{text}");
        }
    }

    #[test]
    fn malformed_or_overflowing_text_is_rejected() {
        for text in [
            "",
            ".",
            "-",
            "1e-5",
            "1,5",
            "abc",
            "1.2.3",
            "+1",
            "9223372036855",
        ] {
            assert_eq!(parse_usd(text), None, "{text}");
        }
    }

    #[test]
    fn json_numbers_keep_every_decimal_they_were_written_with() {
        let cases = [
            ("0", 0),
            ("4", 4_000_000),
            ("6.5", 6_500_000),
            ("0.1", 100_000),
            ("0.2", 200_000),
            ("25.123456", 25_123_456),
            ("0.000123", 123),
            ("1e-6", 1),
            ("99999.999999", 99_999_999_999),
            ("-3.75", -3_750_000),
        ];
        for (json, micros) in cases {
            assert_eq!(usd(json).unwrap(), Usd(MicroUsd(micros)), "{json}");
        }
        assert_eq!(usd("\"42.1\"").unwrap(), Usd(MicroUsd(42_100_000)));
    }

    #[test]
    fn amounts_that_do_not_fit_are_errors() {
        assert!(usd("18446744073709551615").is_err());
        assert!(usd("1e300").is_err());
        assert!(usd("\"lots\"").is_err());
        assert!(usd("true").is_err());
    }
}
