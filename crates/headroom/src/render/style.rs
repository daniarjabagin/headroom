use std::ffi::OsStr;

use headroom_core::pace::Tone;

use super::printable::printable;

const RESET: &str = "\x1b[0m";
const BOLD: &str = "\x1b[1m";
const DIM: &str = "\x1b[2m";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgb(u8, u8, u8);

const ACCENT: Rgb = Rgb(0x00, 0x91, 0xFF);
const AMBER: Rgb = Rgb(0xFF, 0xD6, 0x0A);
const RED: Rgb = Rgb(0xFF, 0x45, 0x3A);
const GRAY: Rgb = Rgb(0x8E, 0x8E, 0x93);
const TRACK: Rgb = Rgb(0x55, 0x55, 0x58);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Palette {
    color: bool,
}

impl Palette {
    #[cfg(test)]
    pub fn plain() -> Palette {
        Palette { color: false }
    }

    #[cfg(test)]
    pub fn colored() -> Palette {
        Palette { color: true }
    }

    pub fn detect(no_color: Option<&OsStr>, is_terminal: bool) -> Palette {
        let disabled = no_color.is_some_and(|value| !value.is_empty());
        Palette {
            color: is_terminal && !disabled,
        }
    }

    pub fn is_colored(self) -> bool {
        self.color
    }

    pub fn tone(self, text: &str, tone: Tone) -> String {
        self.rgb(text, tone_color(tone))
    }

    pub fn track(self, text: &str) -> String {
        self.rgb(text, TRACK)
    }

    pub fn bold(self, text: &str) -> String {
        self.wrap(text, BOLD)
    }

    pub fn dim(self, text: &str) -> String {
        self.wrap(text, DIM)
    }

    fn rgb(self, text: &str, Rgb(r, g, b): Rgb) -> String {
        self.wrap(text, &format!("\x1b[38;2;{r};{g};{b}m"))
    }

    fn wrap(self, text: &str, code: &str) -> String {
        let text = printable(text);
        if self.color && !text.is_empty() {
            format!("{code}{text}{RESET}")
        } else {
            text.into_owned()
        }
    }
}

pub fn tone_hex(tone: Tone) -> String {
    let Rgb(r, g, b) = tone_color(tone);
    format!("#{r:02x}{g:02x}{b:02x}")
}

fn tone_color(tone: Tone) -> Rgb {
    match tone {
        Tone::Good => ACCENT,
        Tone::Warning => AMBER,
        Tone::Critical => RED,
        Tone::Neutral => GRAY,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_palette_leaves_text_untouched() {
        let palette = Palette::plain();
        assert_eq!(palette.tone("62%", Tone::Critical), "62%");
        assert_eq!(palette.bold("Codex"), "Codex");
    }

    #[test]
    fn colored_palette_uses_truecolor_by_tone() {
        let palette = Palette::colored();
        assert_eq!(
            palette.tone("x", Tone::Good),
            "\x1b[38;2;0;145;255mx\x1b[0m"
        );
        assert_eq!(
            palette.tone("x", Tone::Warning),
            "\x1b[38;2;255;214;10mx\x1b[0m"
        );
        assert_eq!(palette.tone("", Tone::Warning), "");
    }

    #[test]
    fn tone_hex_matches_the_terminal_colors() {
        assert_eq!(tone_hex(Tone::Warning), "#ffd60a");
        assert_eq!(tone_hex(Tone::Critical), "#ff453a");
    }

    #[test]
    fn no_color_or_a_pipe_disables_color() {
        assert!(Palette::detect(None, true).is_colored());
        assert!(!Palette::detect(Some(OsStr::new("1")), true).is_colored());
        assert!(Palette::detect(Some(OsStr::new("")), true).is_colored());
        assert!(!Palette::detect(None, false).is_colored());
    }
}
