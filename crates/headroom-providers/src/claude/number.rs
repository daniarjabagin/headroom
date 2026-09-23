use serde_json::Number;

const LARGEST_EXACT_F64_INTEGER: f64 = 9_007_199_254_740_992.0;

pub(super) fn whole_number(number: &Number) -> Option<i64> {
    number
        .as_i64()
        .or_else(|| number.as_f64().and_then(exact_integer))
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::float_cmp,
    reason = "an exact whole-number check on a value inside the range f64 represents exactly"
)]
fn exact_integer(value: f64) -> Option<i64> {
    let whole = value.is_finite() && value.trunc() == value;
    (whole && value.abs() <= LARGEST_EXACT_F64_INTEGER).then_some(value as i64)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn number(text: &str) -> Number {
        serde_json::from_str(text).unwrap()
    }

    #[test]
    fn integers_and_whole_floats_convert() {
        assert_eq!(whole_number(&number("1250")), Some(1250));
        assert_eq!(whole_number(&number("-3")), Some(-3));
        assert_eq!(
            whole_number(&number("1760000000000.0")),
            Some(1_760_000_000_000)
        );
    }

    #[test]
    fn fractions_and_huge_values_are_rejected() {
        assert_eq!(whole_number(&number("12.5")), None);
        assert_eq!(whole_number(&number("18446744073709551615")), None);
        assert_eq!(whole_number(&number("1e300")), None);
    }
}
