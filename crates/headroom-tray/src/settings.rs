use serde::Deserialize;
use serde_json::json;

use crate::payload::{Display, ResetFormat, ValueMode};

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct MotionSettings {
    reduced_motion: bool,
}

#[must_use]
pub fn reduced_motion_setting(settings_json: &str) -> bool {
    serde_json::from_str::<MotionSettings>(settings_json)
        .is_ok_and(|settings| settings.reduced_motion)
}

#[must_use]
pub fn toggled_value_mode_patch(display: &Display) -> String {
    let next = match display.value_mode {
        ValueMode::Left => "used",
        ValueMode::Used => "left",
    };
    json!({ "display": { "value_mode": next } }).to_string()
}

#[must_use]
pub fn toggled_reset_format_patch(display: &Display) -> String {
    let next = match display.reset_format {
        ResetFormat::Countdown => "exact",
        ResetFormat::Exact => "countdown",
    };
    json!({ "display": { "reset_format": next } }).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_reduced_motion() {
        assert!(reduced_motion_setting(
            r#"{"reduced_motion":true,"display":{}}"#
        ));
        assert!(!reduced_motion_setting("{}"));
        assert!(!reduced_motion_setting("not json"));
    }

    #[test]
    fn toggle_patches() {
        let display = Display::default();
        assert_eq!(
            toggled_value_mode_patch(&display),
            r#"{"display":{"value_mode":"used"}}"#
        );
        assert_eq!(
            toggled_reset_format_patch(&display),
            r#"{"display":{"reset_format":"exact"}}"#
        );
    }
}
