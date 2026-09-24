use std::f64::consts::TAU;

use crate::format::round_percent;
use crate::palette::Rgba;
use crate::payload::{Headline, Tone, ValueMode};

pub const SIZES: [u32; 4] = [16, 22, 24, 32];
const LINE_RATIO: f64 = 0.2;
const MIN_LINE: f64 = 2.0;
const SAMPLES: u32 = 4;
const TRACK: Rgba = Rgba {
    red: 0.5,
    green: 0.5,
    blue: 0.5,
    alpha: 0.45,
};

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

    fn fraction(self) -> f64 {
        f64::from(self.percent) / 100.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pixmap {
    pub size: u32,
    pub argb: Vec<u8>,
}

struct Ring {
    center: f64,
    radius: f64,
    half_line: f64,
    sweep: f64,
}

impl Ring {
    fn new(size: u32, fraction: f64) -> Self {
        let size = f64::from(size);
        let line = (size * LINE_RATIO).max(MIN_LINE);
        Self {
            center: size / 2.0,
            radius: (size - line) / 2.0,
            half_line: line / 2.0,
            sweep: fraction.clamp(0.0, 1.0) * TAU,
        }
    }

    fn on_track(&self, x: f64, y: f64) -> bool {
        let distance = (x - self.center).hypot(y - self.center);
        (distance - self.radius).abs() <= self.half_line
    }

    fn on_arc(&self, x: f64, y: f64) -> bool {
        if self.sweep <= 0.0 {
            return false;
        }
        let (dx, dy) = (x - self.center, y - self.center);
        let angle = dx.atan2(-dy).rem_euclid(TAU);
        let within = self.on_track(x, y) && (angle <= self.sweep || self.sweep >= TAU);
        within || self.near_cap(x, y, 0.0) || self.near_cap(x, y, self.sweep)
    }

    fn near_cap(&self, x: f64, y: f64, angle: f64) -> bool {
        let cap_x = self.center + self.radius * angle.sin();
        let cap_y = self.center - self.radius * angle.cos();
        (x - cap_x).hypot(y - cap_y) <= self.half_line
    }
}

fn coverage(ring: &Ring, px: u32, py: u32) -> (f64, f64) {
    let step = 1.0 / f64::from(SAMPLES);
    let (mut track, mut arc) = (0u32, 0u32);
    for sy in 0..SAMPLES {
        for sx in 0..SAMPLES {
            let x = f64::from(px) + (f64::from(sx) + 0.5) * step;
            let y = f64::from(py) + (f64::from(sy) + 0.5) * step;
            if ring.on_arc(x, y) {
                arc += 1;
            } else if ring.on_track(x, y) {
                track += 1;
            }
        }
    }
    let total = f64::from(SAMPLES * SAMPLES);
    (f64::from(track) / total, f64::from(arc) / total)
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the value is clamped to 0..=255 first"
)]
fn to_byte(value: f64) -> u8 {
    (value.clamp(0.0, 1.0) * 255.0).round() as u8
}

fn blend(track: f64, arc: f64, color: Rgba) -> [u8; 4] {
    let arc_alpha = arc * color.alpha;
    let track_alpha = track * TRACK.alpha;
    let alpha = arc_alpha + track_alpha;
    if alpha <= 0.0 {
        return [0; 4];
    }
    let mix = |arc_channel: f64, track_channel: f64| {
        to_byte((arc_channel * arc_alpha + track_channel * track_alpha) / alpha)
    };
    [
        to_byte(alpha),
        mix(color.red, TRACK.red),
        mix(color.green, TRACK.green),
        mix(color.blue, TRACK.blue),
    ]
}

#[must_use]
pub fn ring_pixmap(size: u32, key: RingKey, color: Rgba) -> Pixmap {
    let ring = Ring::new(size, key.fraction());
    let mut argb = Vec::with_capacity((size * size * 4) as usize);
    for y in 0..size {
        for x in 0..size {
            let (track, arc) = coverage(&ring, x, y);
            argb.extend_from_slice(&blend(track, arc, color));
        }
    }
    Pixmap { size, argb }
}

#[must_use]
pub fn ring_pixmaps(key: RingKey, color: Rgba) -> Vec<Pixmap> {
    SIZES
        .iter()
        .map(|size| ring_pixmap(*size, key, color))
        .collect()
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

    const RED: Rgba = Rgba {
        red: 1.0,
        green: 0.0,
        blue: 0.0,
        alpha: 1.0,
    };

    fn pixel(pixmap: &Pixmap, x: u32, y: u32) -> [u8; 4] {
        let offset = ((y * pixmap.size + x) * 4) as usize;
        pixmap.argb[offset..offset + 4].try_into().unwrap()
    }

    fn key(percent: u8) -> RingKey {
        RingKey {
            percent,
            tone: Tone::Critical,
        }
    }

    #[test]
    fn pixmaps_have_argb_bytes_for_every_size() {
        let pixmaps = ring_pixmaps(key(50), RED);
        assert_eq!(pixmaps.len(), SIZES.len());
        for pixmap in pixmaps {
            assert_eq!(pixmap.argb.len(), (pixmap.size * pixmap.size * 4) as usize);
        }
    }

    #[test]
    fn half_ring_colors_the_right_side_only() {
        let pixmap = ring_pixmap(32, key(50), RED);
        let right = pixel(&pixmap, 29, 16);
        let left = pixel(&pixmap, 2, 16);
        assert_eq!(right, [255, 255, 0, 0]);
        assert_eq!(left[0], to_byte(TRACK.alpha));
        assert_eq!(left[1], left[2]);
        assert_eq!(pixel(&pixmap, 16, 16), [0; 4]);
    }

    #[test]
    fn empty_and_full_rings() {
        let empty = ring_pixmap(22, key(0), RED);
        assert!(empty.argb.chunks(4).all(|p| p[1] == p[2]));
        let full = ring_pixmap(22, key(100), RED);
        assert_eq!(pixel(&full, 2, 11), [255, 255, 0, 0]);
    }

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
