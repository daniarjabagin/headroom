use headroom_core::units::MicroUsd;
use serde::Serialize;

use crate::error::PricingError;

const USD_TO_PICO_DIGITS: usize = 12;
const PER_MILLION_TO_PICO_DIGITS: usize = 6;
const PPM_DIGITS: usize = 6;
const PPM: u128 = 1_000_000;
const ATTO_PER_MICRO: u128 = 1_000_000_000_000;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
pub struct PicoUsd(pub u64);

impl PicoUsd {
    pub(crate) fn from_usd(field: &str, value: f64) -> Result<PicoUsd, PricingError> {
        scaled_decimal(field, value, USD_TO_PICO_DIGITS).map(PicoUsd)
    }

    pub(crate) fn from_usd_per_million(field: &str, value: f64) -> Result<PicoUsd, PricingError> {
        scaled_decimal(field, value, PER_MILLION_TO_PICO_DIGITS).map(PicoUsd)
    }

    pub(crate) fn scaled(self, multiplier: Multiplier) -> PicoUsd {
        let product = u128::from(self.0) * u128::from(multiplier.0);
        let rounded = (product + PPM / 2) / PPM;
        PicoUsd(u64::try_from(rounded).unwrap_or(u64::MAX))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
pub struct Multiplier(u64);

impl Multiplier {
    pub const ONE: Multiplier = Multiplier(1_000_000);

    #[must_use]
    pub const fn from_ppm(ppm: u64) -> Multiplier {
        Multiplier(ppm)
    }

    #[must_use]
    pub fn ppm(self) -> u64 {
        self.0
    }

    pub(crate) fn parse(field: &str, value: f64) -> Result<Multiplier, PricingError> {
        scaled_decimal(field, value, PPM_DIGITS).map(Multiplier)
    }
}

pub(crate) fn round_atto_to_micro(atto: u128) -> Option<MicroUsd> {
    let micro = atto.checked_add(ATTO_PER_MICRO / 2)? / ATTO_PER_MICRO;
    i64::try_from(micro).ok().map(MicroUsd)
}

pub(crate) fn pico_to_atto_ppm(pico: u128, multiplier: Multiplier) -> Option<u128> {
    pico.checked_mul(u128::from(multiplier.0))
}

pub(crate) fn pico_to_atto(pico: u128) -> Option<u128> {
    pico.checked_mul(PPM)
}

fn scaled_decimal(field: &str, value: f64, digits: usize) -> Result<u64, PricingError> {
    let invalid = || PricingError::InvalidRate {
        field: field.to_owned(),
        value,
    };
    if !value.is_finite() || value < 0.0 {
        return Err(invalid());
    }
    let text = format!("{value}");
    let (whole, fraction) = text.split_once('.').unwrap_or((text.as_str(), ""));
    let kept: String = fraction
        .chars()
        .chain(std::iter::repeat('0'))
        .take(digits)
        .collect();
    let round_up = fraction.chars().nth(digits).is_some_and(|c| c >= '5');
    let truncated: u64 = format!("{whole}{kept}").parse().map_err(|_| invalid())?;
    truncated
        .checked_add(u64::from(round_up))
        .ok_or_else(invalid)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn per_token_floats_convert_exactly() {
        let cases = [
            (1.25e-06, 1_250_000),
            (5e-06, 5_000_000),
            (1.75e-07, 175_000),
            (5e-09, 5_000),
            (0.00012, 120_000_000),
            (0.01, 10_000_000_000),
            (0.0, 0),
        ];
        for (value, pico) in cases {
            assert_eq!(
                PicoUsd::from_usd("f", value).unwrap(),
                PicoUsd(pico),
                "{value}"
            );
        }
    }

    #[test]
    fn per_million_floats_convert_with_half_up_rounding() {
        let cases = [
            (0.125, 125_000),
            (0.199_999_999_999_999_98, 200_000),
            (6.25, 6_250_000),
            (0.000_001_4, 1),
            (0.000_000_4, 0),
        ];
        for (value, pico) in cases {
            assert_eq!(
                PicoUsd::from_usd_per_million("f", value).unwrap(),
                PicoUsd(pico),
                "{value}"
            );
        }
    }

    #[test]
    fn negative_and_non_finite_rates_are_rejected() {
        for value in [-1.0, f64::NAN, f64::INFINITY, 1e300] {
            assert!(PicoUsd::from_usd("f", value).is_err(), "{value}");
        }
    }

    #[test]
    fn multipliers_parse_to_parts_per_million() {
        assert_eq!(Multiplier::parse("m", 2.5).unwrap(), Multiplier(2_500_000));
        assert_eq!(Multiplier::parse("m", 1.0).unwrap(), Multiplier::ONE);
        assert_eq!(Multiplier::parse("m", 1.8).unwrap(), Multiplier(1_800_000));
    }

    #[test]
    fn scaling_rounds_half_up() {
        assert_eq!(PicoUsd(3).scaled(Multiplier(1_250_000)), PicoUsd(4));
        assert_eq!(PicoUsd(2).scaled(Multiplier(1_250_000)), PicoUsd(3));
        assert_eq!(PicoUsd(1).scaled(Multiplier(1_250_000)), PicoUsd(1));
        assert_eq!(
            PicoUsd(1_000_000).scaled(Multiplier(100_000)),
            PicoUsd(100_000)
        );
    }

    #[test]
    fn atto_rounds_half_up_to_micro() {
        assert_eq!(round_atto_to_micro(0), Some(MicroUsd(0)));
        assert_eq!(round_atto_to_micro(499_999_999_999), Some(MicroUsd(0)));
        assert_eq!(round_atto_to_micro(500_000_000_000), Some(MicroUsd(1)));
        assert_eq!(round_atto_to_micro(1_499_999_999_999), Some(MicroUsd(1)));
        assert_eq!(round_atto_to_micro(u128::MAX), None);
    }
}
