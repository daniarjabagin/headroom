use crate::palette::{Palette, PaletteError};

const TEMPLATES: [&str; 5] = [
    include_str!("style/base.css"),
    include_str!("style/cards.css"),
    include_str!("style/features.css"),
    include_str!("style/compact.css"),
    include_str!("style/sheen.css"),
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
    fn the_sheen_sweeps_only_while_running_and_with_motion() {
        let css = stylesheet(&Palette::load(Scheme::Light).unwrap()).unwrap();
        assert!(css.contains("@keyframes headroom-sheen"));
        assert!(css.contains(".headroom-meter.sheen-running .headroom-sheen"));
        assert!(css.contains(".headroom-popup.reduced-motion .headroom-meter .headroom-sheen"));
        assert!(css.contains("alpha(#ffffff, 0.42) 50%"));
    }

    #[test]
    fn schemes_differ() {
        let light = stylesheet(&Palette::load(Scheme::Light).unwrap()).unwrap();
        let dark = stylesheet(&Palette::load(Scheme::Dark).unwrap()).unwrap();
        assert!(light.contains("background-color: #ffffff"));
        assert!(dark.contains("background-color: #1e1e1e"));
    }
}
