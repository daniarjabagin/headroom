use serde::Deserialize;

const U64_LIMIT: f64 = 18_446_744_073_709_551_616.0;

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
pub(super) enum FlexNumber {
    Number(serde_json::Number),
    Text(String),
}

impl FlexNumber {
    pub(super) fn as_f64(&self) -> Option<f64> {
        let value = match self {
            FlexNumber::Number(number) => number.as_f64()?,
            FlexNumber::Text(text) => text.trim().parse::<f64>().ok()?,
        };
        value.is_finite().then_some(value)
    }

    pub(super) fn whole(&self) -> Option<u64> {
        match self {
            FlexNumber::Number(number) => number.as_u64().or_else(|| floor_to_u64(self.as_f64()?)),
            FlexNumber::Text(text) => text
                .trim()
                .parse::<u64>()
                .ok()
                .or_else(|| floor_to_u64(self.as_f64()?)),
        }
    }
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "value is floored and range-checked to fit u64 first"
)]
fn floor_to_u64(value: f64) -> Option<u64> {
    let floored = value.floor();
    (0.0..U64_LIMIT)
        .contains(&floored)
        .then_some(floored as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flex(json: &str) -> FlexNumber {
        serde_json::from_str(json).unwrap()
    }

    #[test]
    fn numbers_and_numeric_strings_read_as_f64() {
        assert_eq!(flex("37.5").as_f64(), Some(37.5));
        assert_eq!(flex("\"62\"").as_f64(), Some(62.0));
        assert_eq!(flex("\" 1.5 \"").as_f64(), Some(1.5));
        assert_eq!(flex("\"n/a\"").as_f64(), None);
    }

    #[test]
    fn whole_floors_and_rejects_negative() {
        assert_eq!(flex("821.9").whole(), Some(821));
        assert_eq!(flex("\"0\"").whole(), Some(0));
        assert_eq!(flex("\"12.7\"").whole(), Some(12));
        assert_eq!(flex("-3").whole(), None);
        assert_eq!(flex("18446744073709551615").whole(), Some(u64::MAX));
    }
}
