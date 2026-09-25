#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "each flag is an independent modifier key"
)]
pub struct Modifiers {
    pub shift: bool,
    pub control: bool,
    pub alt: bool,
    pub super_key: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Accelerator {
    pub modifiers: Modifiers,
    pub key: String,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AcceleratorError {
    #[error("the shortcut {0} has an unclosed modifier")]
    Unclosed(String),
    #[error("the shortcut modifier <{0}> is not supported")]
    UnknownModifier(String),
    #[error("the shortcut {0} has no key")]
    MissingKey(String),
    #[error("the shortcut key {0} is not a key name")]
    InvalidKey(String),
}

fn set_modifier(modifiers: &mut Modifiers, name: &str) -> Result<(), AcceleratorError> {
    let flag = match name.to_ascii_lowercase().as_str() {
        "shift" => &mut modifiers.shift,
        "control" | "ctrl" | "primary" => &mut modifiers.control,
        "alt" | "mod1" => &mut modifiers.alt,
        "super" | "meta" => &mut modifiers.super_key,
        _ => return Err(AcceleratorError::UnknownModifier(name.to_owned())),
    };
    *flag = true;
    Ok(())
}

fn key_name(key: &str) -> String {
    if key.chars().count() == 1 {
        key.to_ascii_lowercase()
    } else {
        key.to_owned()
    }
}

pub fn parse(text: &str) -> Result<Option<Accelerator>, AcceleratorError> {
    let text = text.trim();
    if text.is_empty() {
        return Ok(None);
    }
    let mut modifiers = Modifiers::default();
    let mut rest = text;
    while let Some(group) = rest.strip_prefix('<') {
        let (name, after) = group
            .split_once('>')
            .ok_or_else(|| AcceleratorError::Unclosed(text.to_owned()))?;
        set_modifier(&mut modifiers, name)?;
        rest = after;
    }
    if rest.is_empty() {
        return Err(AcceleratorError::MissingKey(text.to_owned()));
    }
    if !rest.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err(AcceleratorError::InvalidKey(rest.to_owned()));
    }
    Ok(Some(Accelerator {
        modifiers,
        key: key_name(rest),
    }))
}

impl Accelerator {
    #[must_use]
    pub fn portal_trigger(&self) -> String {
        let modifiers = self.modifiers;
        let names = [
            (modifiers.control, "CTRL"),
            (modifiers.alt, "ALT"),
            (modifiers.shift, "SHIFT"),
            (modifiers.super_key, "LOGO"),
        ];
        names
            .iter()
            .filter(|(on, _)| *on)
            .map(|(_, name)| *name)
            .chain([self.key.as_str()])
            .collect::<Vec<_>>()
            .join("+")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parsed(text: &str) -> Accelerator {
        parse(text).unwrap().unwrap()
    }

    #[test]
    fn parses_gtk_accelerators() {
        let super_u = parsed("<Super>u");
        assert!(super_u.modifiers.super_key);
        assert_eq!(super_u.key, "u");
        let ctrl_alt = parsed("<Ctrl><Alt>H");
        assert_eq!(
            ctrl_alt.modifiers,
            Modifiers {
                control: true,
                alt: true,
                ..Modifiers::default()
            }
        );
        assert_eq!(ctrl_alt.key, "h");
        let function = parsed("<Primary><Shift>F5");
        assert!(function.modifiers.control && function.modifiers.shift);
        assert_eq!(function.key, "F5");
        assert!(parsed("<Meta>space").modifiers.super_key);
    }

    #[test]
    fn empty_disables_the_shortcut() {
        assert_eq!(parse(""), Ok(None));
        assert_eq!(parse("  "), Ok(None));
    }

    #[test]
    fn rejects_broken_accelerators() {
        assert_eq!(
            parse("<Super"),
            Err(AcceleratorError::Unclosed("<Super".into()))
        );
        assert_eq!(
            parse("<Hyper>u"),
            Err(AcceleratorError::UnknownModifier("Hyper".into()))
        );
        assert_eq!(
            parse("<Super>"),
            Err(AcceleratorError::MissingKey("<Super>".into()))
        );
        assert_eq!(
            parse("<Super>a+b"),
            Err(AcceleratorError::InvalidKey("a+b".into()))
        );
    }

    #[test]
    fn portal_triggers_use_xdg_names() {
        assert_eq!(parsed("<Super>u").portal_trigger(), "LOGO+u");
        assert_eq!(
            parsed("<Shift><Alt><Control>Return").portal_trigger(),
            "CTRL+ALT+SHIFT+Return"
        );
        assert_eq!(parsed("F9").portal_trigger(), "F9");
    }
}
