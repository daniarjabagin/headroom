use std::fmt;
use std::iter::Sum;
use std::ops::{Add, AddAssign};

use serde::{Deserialize, Serialize};

#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct Tokens(pub u64);

impl Tokens {
    pub const ZERO: Tokens = Tokens(0);

    #[must_use]
    pub fn saturating_sub(self, other: Tokens) -> Tokens {
        Tokens(self.0.saturating_sub(other.0))
    }
}

impl Add for Tokens {
    type Output = Tokens;

    fn add(self, other: Tokens) -> Tokens {
        Tokens(self.0.saturating_add(other.0))
    }
}

impl AddAssign for Tokens {
    fn add_assign(&mut self, other: Tokens) {
        *self = *self + other;
    }
}

impl Sum for Tokens {
    fn sum<I: Iterator<Item = Tokens>>(iter: I) -> Tokens {
        iter.fold(Tokens::ZERO, Add::add)
    }
}

#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct MicroUsd(pub i64);

impl MicroUsd {
    pub const ZERO: MicroUsd = MicroUsd(0);
}

impl Add for MicroUsd {
    type Output = MicroUsd;

    fn add(self, other: MicroUsd) -> MicroUsd {
        MicroUsd(self.0.saturating_add(other.0))
    }
}

impl AddAssign for MicroUsd {
    fn add_assign(&mut self, other: MicroUsd) {
        *self = *self + other;
    }
}

impl Sum for MicroUsd {
    fn sum<I: Iterator<Item = MicroUsd>>(iter: I) -> MicroUsd {
        iter.fold(MicroUsd::ZERO, Add::add)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct CurrencyCode(String);

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{0:?} is not an ISO 4217 currency code")]
pub struct InvalidCurrencyCode(pub String);

impl CurrencyCode {
    pub fn parse(text: &str) -> Result<CurrencyCode, InvalidCurrencyCode> {
        let valid = text.len() == 3 && text.bytes().all(|b| b.is_ascii_alphabetic());
        if valid {
            Ok(CurrencyCode(text.to_ascii_uppercase()))
        } else {
            Err(InvalidCurrencyCode(text.to_owned()))
        }
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for CurrencyCode {
    type Error = InvalidCurrencyCode;

    fn try_from(text: String) -> Result<CurrencyCode, InvalidCurrencyCode> {
        CurrencyCode::parse(&text)
    }
}

impl From<CurrencyCode> for String {
    fn from(code: CurrencyCode) -> String {
        code.0
    }
}

impl fmt::Display for CurrencyCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// An amount in millionths of one unit of `currency`; amounts in different currencies never mix.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Money {
    pub currency: CurrencyCode,
    pub micros: i64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(from = "f64", into = "f64")]
pub struct Percent(f64);

impl Percent {
    pub const ZERO: Percent = Percent(0.0);
    pub const FULL: Percent = Percent(100.0);

    #[must_use]
    pub fn new(value: f64) -> Percent {
        if value.is_nan() || value < 0.0 {
            Percent(0.0)
        } else {
            Percent(value)
        }
    }

    #[must_use]
    pub fn value(self) -> f64 {
        self.0
    }

    #[must_use]
    pub fn remaining(self) -> Percent {
        Percent((100.0 - self.0).max(0.0))
    }
}

impl From<f64> for Percent {
    fn from(value: f64) -> Percent {
        Percent::new(value)
    }
}

impl From<Percent> for f64 {
    fn from(percent: Percent) -> f64 {
        percent.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokens_add_saturates() {
        assert_eq!(Tokens(u64::MAX) + Tokens(1), Tokens(u64::MAX));
        let mut total = Tokens(2);
        total += Tokens(3);
        assert_eq!(total, Tokens(5));
    }

    #[test]
    fn tokens_sum_and_saturating_sub() {
        let total: Tokens = [Tokens(1), Tokens(2), Tokens(3)].into_iter().sum();
        assert_eq!(total, Tokens(6));
        assert_eq!(Tokens(3).saturating_sub(Tokens(5)), Tokens::ZERO);
    }

    #[test]
    fn micro_usd_add_saturates_both_ways() {
        assert_eq!(MicroUsd(i64::MAX) + MicroUsd(1), MicroUsd(i64::MAX));
        assert_eq!(MicroUsd(i64::MIN) + MicroUsd(-1), MicroUsd(i64::MIN));
        let total: MicroUsd = [MicroUsd(10), MicroUsd(-4)].into_iter().sum();
        assert_eq!(total, MicroUsd(6));
    }

    #[test]
    fn percent_clamps_nan_and_negative() {
        assert_eq!(Percent::new(f64::NAN), Percent::ZERO);
        assert_eq!(Percent::new(-3.0), Percent::ZERO);
        assert_eq!(Percent::from(130.0), Percent::new(130.0));
        assert!(Percent::new(130.0) > Percent::FULL);
    }

    #[test]
    fn percent_remaining_never_negative() {
        assert_eq!(Percent::new(62.0).remaining(), Percent::new(38.0));
        assert_eq!(Percent::new(130.0).remaining(), Percent::ZERO);
    }

    #[test]
    fn percent_serde_goes_through_new() {
        let parsed: Percent = serde_json::from_str("-5.0").unwrap();
        assert_eq!(parsed, Percent::ZERO);
        assert_eq!(serde_json::to_string(&Percent::new(12.5)).unwrap(), "12.5");
    }

    #[test]
    fn currency_codes_are_three_letters_in_upper_case() {
        assert_eq!(CurrencyCode::parse("cny").unwrap().as_str(), "CNY");
        assert_eq!(CurrencyCode::parse("USD").unwrap().to_string(), "USD");
        for text in ["", "US", "USDT", "U5D", "¥¥¥", " CN"] {
            assert_eq!(
                CurrencyCode::parse(text),
                Err(InvalidCurrencyCode(text.to_owned())),
                "{text}"
            );
        }
    }

    #[test]
    fn money_serializes_its_currency_as_a_code() {
        let money = Money {
            currency: CurrencyCode::parse("CNY").unwrap(),
            micros: 12_500_000,
        };
        let json = serde_json::to_string(&money).unwrap();
        assert_eq!(json, r#"{"currency":"CNY","micros":12500000}"#);
        assert_eq!(serde_json::from_str::<Money>(&json).unwrap(), money);
        let bad = serde_json::from_str::<Money>(r#"{"currency":"yuan","micros":1}"#);
        assert!(bad.is_err());
    }
}
