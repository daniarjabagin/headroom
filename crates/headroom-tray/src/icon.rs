mod canvas;
mod glyph;
mod label;
mod layout;
mod mark;
mod painter;
mod pulse;
mod shapes;
mod tints;

pub use glyph::{Gauge, Glyph};
pub use label::TrayText;
pub use layout::FULL_OPACITY;
pub use painter::{Painter, Pixmaps, TrayIcon};
pub use pulse::{FRAME, opacity};
pub use tints::Tints;

use crate::format::round_percent;
use crate::payload::{Headline, Tone, ValueMode};

pub const SIZES: [u32; 6] = [16, 22, 24, 32, 48, 64];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RingKey {
    pub percent: u8,
    pub tone: Tone,
}

impl RingKey {
    #[must_use]
    pub fn from_headline(headline: &Headline, mode: ValueMode) -> Self {
        let value = match mode {
            ValueMode::Left => headline.remaining_percent,
            ValueMode::Used => headline.used_percent,
        };
        let percent = u8::try_from(round_percent(value).min(100)).unwrap_or(100);
        Self {
            percent,
            tone: headline.tone,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pixmap {
    pub size: u32,
    pub argb: Vec<u8>,
}

#[must_use]
pub fn pixmap_from_rgba(size: u32, rgba: &[u8]) -> Pixmap {
    let argb = rgba
        .as_chunks::<4>()
        .0
        .iter()
        .flat_map(|[red, green, blue, alpha]| [*alpha, *red, *green, *blue])
        .collect();
    Pixmap { size, argb }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_rgba_to_network_order_argb() {
        let pixmap = pixmap_from_rgba(1, &[10, 20, 30, 40]);
        assert_eq!(pixmap.argb, [40, 10, 20, 30]);
    }

    #[test]
    fn key_follows_the_value_mode() {
        let headline: Headline = serde_json::from_value(serde_json::json!({
            "account_id": "codex:1", "provider": "codex", "provider_name": "Codex",
            "account_label": "work", "window": "weekly", "window_label": "Weekly",
            "used_percent": 26.6, "remaining_percent": 73.4, "tone": "good"
        }))
        .unwrap();
        assert_eq!(
            RingKey::from_headline(&headline, ValueMode::Left).percent,
            73
        );
        assert_eq!(
            RingKey::from_headline(&headline, ValueMode::Used).percent,
            27
        );
    }
}
