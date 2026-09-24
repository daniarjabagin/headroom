use std::collections::BTreeMap;

use serde::Deserialize;

use crate::payload::Tone;

const TOKENS_JSON: &str = include_str!("../../../shell/gnome/styles/tokens.json");
const KNOWN_SERIES: [&str; 19] = [
    "codex",
    "claude",
    "opencode",
    "openrouter",
    "zai",
    "kimi",
    "minimax",
    "grok",
    "cline",
    "devin",
    "copilot",
    "cursor",
    "antigravity",
    "ollama",
    "kilo",
    "warp",
    "poe",
    "deepseek",
    "moonshot",
];
const FALLBACK_SERIES: u32 = 4;

#[derive(Debug, thiserror::Error)]
pub enum PaletteError {
    #[error("unreadable design tokens: {0}")]
    Json(#[from] serde_json::Error),
    #[error("design token {0} is missing")]
    Missing(String),
    #[error("design token {name} has an unreadable color {value}")]
    Color { name: String, value: String },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rgba {
    pub red: f64,
    pub green: f64,
    pub blue: f64,
    pub alpha: f64,
}

impl Rgba {
    fn parse(text: &str) -> Option<Self> {
        let text = text.trim();
        if let Some(hex) = text.strip_prefix('#') {
            return parse_hex(hex);
        }
        let inner = text.strip_prefix("rgba(")?.strip_suffix(')')?;
        let parts: Vec<f64> = inner
            .split(',')
            .map(|part| part.trim().parse::<f64>().ok())
            .collect::<Option<_>>()?;
        match parts.as_slice() {
            [r, g, b, a] => Some(Self::from_bytes(*r, *g, *b, *a)),
            _ => None,
        }
    }

    fn from_bytes(red: f64, green: f64, blue: f64, alpha: f64) -> Self {
        Self {
            red: red / 255.0,
            green: green / 255.0,
            blue: blue / 255.0,
            alpha,
        }
    }
}

fn parse_hex(hex: &str) -> Option<Rgba> {
    if hex.len() != 6 {
        return None;
    }
    let channel =
        |range: std::ops::Range<usize>| u8::from_str_radix(hex.get(range)?, 16).ok().map(f64::from);
    Some(Rgba::from_bytes(
        channel(0..2)?,
        channel(2..4)?,
        channel(4..6)?,
        1.0,
    ))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scheme {
    Light,
    Dark,
}

#[derive(Debug, Deserialize)]
struct TokenFile {
    light: BTreeMap<String, String>,
    dark: BTreeMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct Palette {
    scheme: Scheme,
    tokens: BTreeMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct Palettes {
    pub light: Palette,
    pub dark: Palette,
}

impl Palettes {
    pub fn load() -> Result<Self, PaletteError> {
        Ok(Self {
            light: Palette::load(Scheme::Light)?,
            dark: Palette::load(Scheme::Dark)?,
        })
    }
}

impl Palette {
    pub fn load(scheme: Scheme) -> Result<Self, PaletteError> {
        let file: TokenFile = serde_json::from_str(TOKENS_JSON)?;
        let tokens = match scheme {
            Scheme::Light => file.light,
            Scheme::Dark => file.dark,
        };
        Ok(Self { scheme, tokens })
    }

    #[must_use]
    pub fn scheme(&self) -> Scheme {
        self.scheme
    }

    pub fn css_value(&self, name: &str) -> Result<&str, PaletteError> {
        self.tokens
            .get(name)
            .map(String::as_str)
            .ok_or_else(|| PaletteError::Missing(name.to_owned()))
    }

    pub fn color(&self, name: &str) -> Result<Rgba, PaletteError> {
        let value = self.css_value(name)?;
        Rgba::parse(value).ok_or_else(|| PaletteError::Color {
            name: name.to_owned(),
            value: value.to_owned(),
        })
    }

    pub fn tone(&self, tone: Tone) -> Result<Rgba, PaletteError> {
        self.color(tone_token(tone))
    }

    pub fn series(&self, provider: &str) -> Result<Rgba, PaletteError> {
        self.color(&format!("series-{}", series_key(provider)))
    }
}

#[must_use]
pub fn tone_token(tone: Tone) -> &'static str {
    match tone {
        Tone::Good => "accent",
        Tone::Warning => "warn",
        Tone::Critical => "crit",
        Tone::Neutral => "text-secondary",
    }
}

fn stable_hash(value: &str) -> u32 {
    value.chars().fold(0u32, |hash, c| {
        hash.wrapping_mul(31).wrapping_add(u32::from(c))
    })
}

#[must_use]
pub fn series_key(provider: &str) -> String {
    if KNOWN_SERIES.contains(&provider) {
        provider.to_owned()
    } else {
        format!("other-{}", stable_hash(provider) % FALLBACK_SERIES)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_both_schemes() {
        let light = Palette::load(Scheme::Light).unwrap();
        let dark = Palette::load(Scheme::Dark).unwrap();
        assert_eq!(light.css_value("tray").unwrap(), "#ffffff");
        assert_eq!(dark.css_value("tray").unwrap(), "#1e1e1e");
        assert!(matches!(
            light.css_value("nope"),
            Err(PaletteError::Missing(_))
        ));
    }

    #[test]
    fn parses_hex_and_rgba_colors() {
        let light = Palette::load(Scheme::Light).unwrap();
        let accent = light.color("accent").unwrap();
        assert!((accent.blue - 1.0).abs() < 1e-9);
        assert!(accent.red.abs() < 1e-9);
        let tick = light.color("tick").unwrap();
        assert!((tick.alpha - 0.55).abs() < 1e-9);
        assert!(Rgba::parse("#12345").is_none());
        assert!(Rgba::parse("rgba(1, 2, 3)").is_none());
    }

    #[test]
    fn tone_colors_come_from_tokens() {
        let dark = Palette::load(Scheme::Dark).unwrap();
        assert_eq!(
            dark.tone(Tone::Critical).unwrap(),
            dark.color("crit").unwrap()
        );
        assert_eq!(
            dark.tone(Tone::Good).unwrap(),
            dark.color("accent").unwrap()
        );
    }

    #[test]
    fn every_provider_series_token_is_known() {
        let file: TokenFile = serde_json::from_str(TOKENS_JSON).unwrap();
        for tokens in [&file.light, &file.dark] {
            let named = tokens
                .keys()
                .filter_map(|key| key.strip_prefix("series-"))
                .filter(|key| !key.starts_with("other-"));
            for provider in named {
                assert!(KNOWN_SERIES.contains(&provider), "{provider}");
            }
        }
    }

    #[test]
    fn series_keys_match_the_gnome_hash() {
        assert_eq!(series_key("codex"), "codex");
        assert_eq!(
            series_key("mystery"),
            format!("other-{}", stable_hash("mystery") % 4)
        );
        let light = Palette::load(Scheme::Light).unwrap();
        for provider in KNOWN_SERIES.iter().copied().chain(["unknown-tool"]) {
            assert!(light.series(provider).is_ok(), "{provider}");
        }
    }
}
