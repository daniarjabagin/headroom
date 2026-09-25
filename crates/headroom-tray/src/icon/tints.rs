use super::canvas::to_byte;
use crate::palette::{Palette, PaletteError, Rgba};
use crate::payload::Tone;

const TICK_COLOR: &str = "text";
const TICK_ALPHA: &str = "panel-tick-alpha";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Color(pub [u8; 4]);

impl Color {
    #[must_use]
    pub fn from_rgba(rgba: Rgba) -> Self {
        Self([
            to_byte(rgba.red),
            to_byte(rgba.green),
            to_byte(rgba.blue),
            to_byte(rgba.alpha),
        ])
    }

    #[must_use]
    pub fn rgba(self, opacity: f64) -> Rgba {
        let [red, green, blue, alpha] = self.0.map(|byte| f64::from(byte) / 255.0);
        Rgba {
            red,
            green,
            blue,
            alpha: alpha * opacity,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Tints {
    pub good: Color,
    pub warning: Color,
    pub critical: Color,
    pub neutral: Color,
    pub tick: Color,
}

fn alpha_token(palette: &Palette, name: &str) -> Result<f64, PaletteError> {
    let value = palette.css_value(name)?;
    value.parse::<f64>().map_err(|_| PaletteError::Color {
        name: name.to_owned(),
        value: value.to_owned(),
    })
}

impl Tints {
    pub fn from_palette(palette: &Palette) -> Result<Self, PaletteError> {
        let tone = |tone| palette.tone(tone).map(Color::from_rgba);
        let mut tick = palette.color(TICK_COLOR)?;
        tick.alpha *= alpha_token(palette, TICK_ALPHA)?;
        Ok(Self {
            good: tone(Tone::Good)?,
            warning: tone(Tone::Warning)?,
            critical: tone(Tone::Critical)?,
            neutral: tone(Tone::Neutral)?,
            tick: Color::from_rgba(tick),
        })
    }

    #[must_use]
    pub fn tone(&self, tone: Tone) -> Color {
        match tone {
            Tone::Good => self.good,
            Tone::Warning => self.warning,
            Tone::Critical => self.critical,
            Tone::Neutral => self.neutral,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::palette::{Palettes, Scheme};

    #[test]
    fn tints_come_from_the_design_tokens() {
        let palettes = Palettes::load().unwrap();
        let light = Tints::from_palette(&palettes.light).unwrap();
        assert_eq!(light.warning, Color([0xff, 0xcc, 0x00, 255]));
        assert_eq!(light.critical, Color([0xff, 0x3b, 0x30, 255]));
        assert_eq!(light.tick, Color([0x26, 0x26, 0x26, 204]));
        let dark = Tints::from_palette(&palettes.dark).unwrap();
        assert_eq!(palettes.dark.scheme(), Scheme::Dark);
        assert_eq!(dark.tone(Tone::Critical), Color([0xff, 0x45, 0x3a, 255]));
    }

    #[test]
    fn opacity_scales_alpha() {
        let color = Color([255, 0, 0, 255]).rgba(0.5);
        assert!((color.alpha - 0.5).abs() < 1e-9);
        assert!((color.red - 1.0).abs() < 1e-9);
    }
}
