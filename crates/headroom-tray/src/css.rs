use crate::palette::{Palette, PaletteError};

const TEMPLATES: [&str; 4] = [
    include_str!("style/base.css"),
    include_str!("style/cards.css"),
    include_str!("style/features.css"),
    include_str!("style/compact.css"),
];
const VAR_OPEN: &str = "var(--";

pub fn stylesheet(palette: &Palette) -> Result<String, PaletteError> {
    TEMPLATES
        .iter()
        .map(|template| resolve(template, palette))
        .collect::<Result<Vec<_>, _>>()
        .map(|parts| parts.join("\n"))
}

fn resolve(template: &str, palette: &Palette) -> Result<String, PaletteError> {
    let mut out = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(start) = rest.find(VAR_OPEN) {
        out.push_str(&rest[..start]);
        let after = &rest[start + VAR_OPEN.len()..];
        let end = after
            .find(')')
            .ok_or_else(|| PaletteError::Missing(after.chars().take(20).collect()))?;
        out.push_str(palette.css_value(&after[..end])?);
        rest = &after[end + 1..];
    }
    out.push_str(rest);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::palette::Scheme;

    #[test]
    fn every_variable_resolves_in_both_schemes() {
        for scheme in [Scheme::Light, Scheme::Dark] {
            let css = stylesheet(&Palette::load(scheme).unwrap()).unwrap();
            assert!(!css.contains("var("), "{scheme:?}");
        }
    }

    #[test]
    fn schemes_differ() {
        let light = stylesheet(&Palette::load(Scheme::Light).unwrap()).unwrap();
        let dark = stylesheet(&Palette::load(Scheme::Dark).unwrap()).unwrap();
        assert!(light.contains("background-color: #ffffff"));
        assert!(dark.contains("background-color: #1e1e1e"));
    }
}
