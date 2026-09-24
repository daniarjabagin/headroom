use headroom_core::units::MicroUsd;
use serde::Deserialize;
use serde_json::Number;

use crate::decimal::parse_micros;

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

fn parse_usd(text: &str) -> Option<MicroUsd> {
    parse_micros(text).map(MicroUsd)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn usd(json: &str) -> Result<Usd, serde_json::Error> {
        serde_json::from_str(json)
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
        assert_eq!(usd("\"-0.0000005\"").unwrap(), Usd(MicroUsd(-1)));
    }

    #[test]
    fn amounts_that_do_not_fit_are_errors() {
        assert!(usd("18446744073709551615").is_err());
        assert!(usd("1e300").is_err());
        assert!(usd("\"lots\"").is_err());
        assert!(usd("\"1e-5\"").is_err());
        assert!(usd("true").is_err());
    }
}
