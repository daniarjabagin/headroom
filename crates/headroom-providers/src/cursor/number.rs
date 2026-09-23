use headroom_core::units::MicroUsd;
use jiff::Timestamp;
use serde_json::Value;

const MICROS_PER_CENT_DIGITS: u32 = 4;
const MAX_FRACTION_DIGITS: u32 = 18;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Decimal {
    units: i128,
    scale: u32,
}

impl Decimal {
    pub(super) fn parse(value: &Value) -> Option<Decimal> {
        match value {
            Value::Number(number) => parse_text(&number.to_string()),
            Value::String(text) => parse_text(text.trim()),
            _ => None,
        }
    }

    pub(super) fn is_positive(self) -> bool {
        self.units > 0
    }

    pub(super) fn is_negative(self) -> bool {
        self.units < 0
    }

    pub(super) fn whole(self) -> Option<i64> {
        self.scaled(0)
    }

    pub(super) fn cents_to_micro_usd(self) -> Option<MicroUsd> {
        self.scaled(MICROS_PER_CENT_DIGITS).map(MicroUsd)
    }

    #[allow(
        clippy::cast_precision_loss,
        reason = "percentages are displayed as f64; the value is small and exact enough"
    )]
    pub(super) fn to_f64(self) -> f64 {
        let divisor = 10_f64.powi(i32::try_from(self.scale).unwrap_or(i32::MAX));
        self.units as f64 / divisor
    }

    fn scaled(self, digits: u32) -> Option<i64> {
        let units = if digits >= self.scale {
            self.units
                .checked_mul(10_i128.checked_pow(digits - self.scale)?)?
        } else {
            let divisor = 10_i128.checked_pow(self.scale - digits)?;
            (self.units % divisor == 0).then_some(self.units / divisor)?
        };
        i64::try_from(units).ok()
    }
}

fn parse_text(text: &str) -> Option<Decimal> {
    let (negative, digits) = match text.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, text),
    };
    let (whole, fraction) = digits.split_once('.').unwrap_or((digits, ""));
    let all_digits = |part: &str| part.bytes().all(|b| b.is_ascii_digit());
    if whole.is_empty() || !all_digits(whole) || !all_digits(fraction) {
        return None;
    }
    let scale = u32::try_from(fraction.len()).ok()?;
    if scale > MAX_FRACTION_DIGITS {
        return None;
    }
    let magnitude: i128 = format!("{whole}{fraction}").parse().ok()?;
    let units = if negative { -magnitude } else { magnitude };
    Some(Decimal { units, scale })
}

pub(super) fn epoch_millis(value: &Value) -> Option<Timestamp> {
    Timestamp::from_millisecond(Decimal::parse(value)?.whole()?).ok()
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn decimal(value: &Value) -> Decimal {
        Decimal::parse(value).unwrap()
    }

    #[test]
    fn cents_convert_to_micro_usd_exactly() {
        let cases = [
            (json!(1200), 12_000_000),
            (json!("52692"), 526_920_000),
            (json!(12.5), 125_000),
            (json!("-50000"), -500_000_000),
            (json!(0), 0),
            (json!(0.0001), 1),
        ];
        for (value, micros) in cases {
            assert_eq!(
                decimal(&value).cents_to_micro_usd(),
                Some(MicroUsd(micros)),
                "{value}"
            );
        }
    }

    #[test]
    fn sub_micro_amounts_are_rejected_not_rounded() {
        assert_eq!(decimal(&json!(0.00001)).cents_to_micro_usd(), None);
    }

    #[test]
    fn non_numbers_are_rejected() {
        for value in [
            json!(true),
            json!(null),
            json!("abc"),
            json!("1e5"),
            json!(""),
            json!("-"),
            json!(".5"),
            json!({}),
        ] {
            assert_eq!(Decimal::parse(&value), None, "{value}");
        }
    }

    #[test]
    fn whole_rejects_fractions() {
        assert_eq!(
            decimal(&json!("1770000000000")).whole(),
            Some(1_770_000_000_000)
        );
        assert_eq!(
            decimal(&json!(1_770_000_000_000.0)).whole(),
            Some(1_770_000_000_000)
        );
        assert_eq!(decimal(&json!(1.5)).whole(), None);
    }

    #[test]
    fn percents_become_floats() {
        assert!((decimal(&json!("37.5")).to_f64() - 37.5).abs() < 1e-12);
        assert!((decimal(&json!(125)).to_f64() - 125.0).abs() < 1e-12);
    }

    #[test]
    fn epoch_millis_accept_numbers_and_strings() {
        let expected: Timestamp = "2026-02-02T02:40:00Z".parse().unwrap();
        assert_eq!(epoch_millis(&json!(1_770_000_000_000_i64)), Some(expected));
        assert_eq!(epoch_millis(&json!("1770000000000")), Some(expected));
        assert_eq!(epoch_millis(&json!("soon")), None);
    }
}
