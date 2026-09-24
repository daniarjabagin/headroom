use crate::palette::Scheme;
use crate::payload::Theme;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SystemHints {
    pub supports_color_schemes: bool,
    pub system_dark: bool,
    pub gtk_theme: Option<String>,
}

fn dark_theme_name(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower.ends_with("-dark") || lower.ends_with("_dark") || lower.ends_with(":dark")
}

#[must_use]
pub fn resolve_scheme(theme: Theme, hints: &SystemHints) -> Scheme {
    let dark = match theme {
        Theme::Light => false,
        Theme::Dark => true,
        Theme::System if hints.supports_color_schemes => hints.system_dark,
        Theme::System => hints.gtk_theme.as_deref().is_some_and(dark_theme_name),
    };
    if dark { Scheme::Dark } else { Scheme::Light }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hints(supports: bool, system_dark: bool, theme: Option<&str>) -> SystemHints {
        SystemHints {
            supports_color_schemes: supports,
            system_dark,
            gtk_theme: theme.map(str::to_owned),
        }
    }

    #[test]
    fn forced_themes_win() {
        let dark_system = hints(true, true, None);
        assert_eq!(resolve_scheme(Theme::Light, &dark_system), Scheme::Light);
        assert_eq!(
            resolve_scheme(Theme::Dark, &hints(true, false, None)),
            Scheme::Dark
        );
    }

    #[test]
    fn system_follows_the_portal_when_it_answers() {
        assert_eq!(
            resolve_scheme(Theme::System, &hints(true, true, None)),
            Scheme::Dark
        );
        assert_eq!(
            resolve_scheme(Theme::System, &hints(true, false, Some("Mint-Y-Dark"))),
            Scheme::Light
        );
    }

    #[test]
    fn system_falls_back_to_the_gtk_theme_name() {
        let cases = [
            (Some("Mint-Y-Dark"), Scheme::Dark),
            (Some("Adwaita-dark"), Scheme::Dark),
            (Some("Arc"), Scheme::Light),
            (None, Scheme::Light),
        ];
        for (name, expected) in cases {
            assert_eq!(
                resolve_scheme(Theme::System, &hints(false, false, name)),
                expected
            );
        }
    }
}
