use crate::payload::{Display, ResetFormat, ValueMode};
use crate::preferences::change::Change;

#[must_use]
pub fn toggled_value_mode(display: &Display) -> Change {
    Change::ValueMode(match display.value_mode {
        ValueMode::Left => ValueMode::Used,
        ValueMode::Used => ValueMode::Left,
    })
}

#[must_use]
pub fn toggled_reset_format(display: &Display) -> Change {
    Change::ResetFormat(match display.reset_format {
        ResetFormat::Countdown => ResetFormat::Exact,
        ResetFormat::Exact => ResetFormat::Countdown,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toggles_flip_the_current_value() {
        let display = Display::default();
        assert_eq!(
            toggled_value_mode(&display).patch().to_string(),
            r#"{"display":{"value_mode":"used"}}"#
        );
        assert_eq!(
            toggled_reset_format(&display).patch().to_string(),
            r#"{"display":{"reset_format":"exact"}}"#
        );
        let flipped = Display {
            value_mode: ValueMode::Used,
            reset_format: ResetFormat::Exact,
            ..Display::default()
        };
        assert_eq!(
            toggled_value_mode(&flipped),
            Change::ValueMode(ValueMode::Left)
        );
        assert_eq!(
            toggled_reset_format(&flipped),
            Change::ResetFormat(ResetFormat::Countdown)
        );
    }
}
