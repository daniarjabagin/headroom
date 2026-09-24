use headroom_core::units::MicroUsd;
use serde::Deserialize;
use serde_json::value::RawValue;

const MICROS_PER_USD: i64 = 1_000_000;
const MICRO_DIGITS: usize = 6;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(try_from = "Box<RawValue>")]
pub(super) struct Usd(pub(super) MicroUsd);

impl TryFrom<Box<RawValue>> for Usd {
    type Error = String;

    fn try_from(raw: Box<RawValue>) -> Result<Usd, String> {
        let text = raw.get();
        parse_usd(text)
            .map(Usd)
            .ok_or_else(|| format!("{text} is not a USD amount with at most six decimals"))
    }
}

/// A JSON number written in plain decimal with at most six decimals, converted exactly.
pub(super) fn parse_usd(text: &str) -> Option<MicroUsd> {
    let (negative, unsigned) = match text.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, text),
    };
    let (whole, fraction) = match unsigned.split_once('.') {
        Some((_, "")) => return None,
        Some(parts) => parts,
        None => (unsigned, ""),
    };
    let digits_only = |part: &str| part.bytes().all(|byte| byte.is_ascii_digit());
    let well_formed = !whole.is_empty()
        && fraction.len() <= MICRO_DIGITS
        && digits_only(whole)
        && digits_only(fraction);
    if !well_formed {
        return None;
    }
    let micros = value(whole)?
        .checked_mul(MICROS_PER_USD)?
        .checked_add(value(fraction)?.checked_mul(scale(fraction.len()))?)?;
    Some(MicroUsd(if negative { -micros } else { micros }))
}

fn value(digits: &str) -> Option<i64> {
    digits.bytes().try_fold(0_i64, |total, digit| {
        total.checked_mul(10)?.checked_add(i64::from(digit - b'0'))
    })
}

fn scale(kept_digits: usize) -> i64 {
    (kept_digits..MICRO_DIGITS).fold(1, |total, _| total * 10)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn usd(json: &str) -> Result<Usd, serde_json::Error> {
        serde_json::from_str(json)
    }

    #[test]
    fn json_number_text_converts_exactly() {
        let cases = [
            ("0", 0),
            ("12", 12_000_000),
            ("6.5", 6_500_000),
            ("0.1", 100_000),
            ("0.000001", 1),
            ("18.123456", 18_123_456),
            ("-0.25", -250_000),
            ("9223372036854.775807", i64::MAX),
        ];
        for (json, micros) in cases {
            assert_eq!(usd(json).unwrap(), Usd(MicroUsd(micros)), "{json}");
        }
    }

    #[test]
    fn amounts_that_are_not_exact_micro_usd_are_rejected() {
        for json in [
            "0.0000001",
            "1e-6",
            "1E3",
            "\"4.5\"",
            "true",
            "null",
            "9223372036855",
            "18446744073709551616",
        ] {
            assert!(usd(json).is_err(), "{json}");
        }
        for text in ["", ".", "-", ".5", "7.", "+1", "1.2.3", "--1"] {
            assert_eq!(parse_usd(text), None, "{text}");
        }
    }
}
