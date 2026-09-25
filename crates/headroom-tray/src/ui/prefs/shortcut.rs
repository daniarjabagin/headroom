use crate::preferences::validate::is_valid_shortcut;

const MODIFIER_PREFIXES: [&str; 9] = [
    "Shift",
    "Control",
    "Alt",
    "Super",
    "Meta",
    "Hyper",
    "ISO_",
    "Caps_Lock",
    "Num_Lock",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Capture {
    Wait,
    Cancel,
    Disable,
    Set(String),
}

fn is_modifier(key: &str) -> bool {
    MODIFIER_PREFIXES
        .iter()
        .any(|prefix| key.starts_with(prefix))
}

fn is_function_key(key: &str) -> bool {
    key.strip_prefix('F')
        .is_some_and(|number| !number.is_empty() && number.bytes().all(|b| b.is_ascii_digit()))
}

#[must_use]
pub fn capture(key: &str, modified: bool, accelerator: &str) -> Capture {
    if key.is_empty() || is_modifier(key) {
        return Capture::Wait;
    }
    match (key, modified) {
        ("Escape", false) => Capture::Cancel,
        ("BackSpace", false) => Capture::Disable,
        _ if !modified && !is_function_key(key) => Capture::Wait,
        _ if is_valid_shortcut(accelerator) && !accelerator.is_empty() => {
            Capture::Set(accelerator.to_owned())
        }
        _ => Capture::Wait,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escape_cancels_and_backspace_disables() {
        assert_eq!(capture("Escape", false, "Escape"), Capture::Cancel);
        assert_eq!(capture("BackSpace", false, "BackSpace"), Capture::Disable);
        assert_eq!(
            capture("BackSpace", true, "<Control>BackSpace"),
            Capture::Set("<Control>BackSpace".into())
        );
    }

    #[test]
    fn needs_a_modifier_except_for_function_keys() {
        assert_eq!(capture("u", false, "u"), Capture::Wait);
        assert_eq!(
            capture("u", true, "<Super>u"),
            Capture::Set("<Super>u".into())
        );
        assert_eq!(capture("F12", false, "F12"), Capture::Set("F12".into()));
        assert_eq!(capture("F", false, "F"), Capture::Wait);
    }

    #[test]
    fn waits_while_only_modifiers_are_down() {
        for key in [
            "Super_L",
            "Control_R",
            "Shift_L",
            "Alt_L",
            "ISO_Level3_Shift",
            "",
        ] {
            assert_eq!(capture(key, true, "<Super>"), Capture::Wait, "{key}");
        }
        assert_eq!(capture("u", true, "<Super>u+"), Capture::Wait);
    }
}
