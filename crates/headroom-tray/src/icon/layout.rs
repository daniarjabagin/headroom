use super::canvas::{Group, Layer, Shape, rasterize};
use super::glyph::{Gauge, Glyph};
use super::mark::MARK_GRID;
use super::shapes::{Polygon, Ring, RingArc, RingTrack, RoundedRect};
use super::tints::Tints;
use super::{Pixmap, SIZES};
use crate::palette::Rgba;
use crate::payload::Tone;

pub const FULL_OPACITY: u8 = 255;
const TRACK: Rgba = Rgba {
    red: 0.5,
    green: 0.5,
    blue: 0.5,
    alpha: 0.45,
};
const RING_GAP_RATIO: f64 = 1.0 / 16.0;
const BAR_HEIGHT_RATIO: f64 = 5.0 / 26.0;
const TICK_WIDTH_RATIO: f64 = 2.0 / 26.0;
const TICK_HEIGHT_RATIO: f64 = 9.0 / 26.0;
const MIN_BAR_HEIGHT: f64 = 3.0;
const MIN_TICK_WIDTH: f64 = 1.5;
const BAR_OFFSET_RATIO: f64 = 0.25;

struct Item {
    shapes: Vec<(Box<dyn Shape>, Rgba)>,
    opacity: f64,
}

impl Item {
    fn new(tone: Tone, pulse: u8) -> Self {
        let opacity = if tone == Tone::Critical {
            f64::from(pulse) / 255.0
        } else {
            1.0
        };
        Self {
            shapes: Vec::new(),
            opacity,
        }
    }

    fn with(mut self, shape: impl Shape + 'static, color: Rgba) -> Self {
        self.shapes.push((Box::new(shape), color));
        self
    }
}

struct Frame<'a> {
    size: f64,
    tints: &'a Tints,
    pulse: u8,
}

fn fraction(percent: u8) -> f64 {
    f64::from(percent) / 100.0
}

impl Frame<'_> {
    fn ring(&self, center_x: f64, diameter: f64, gauge: Gauge) -> Item {
        let ring = Ring::new(center_x, self.size / 2.0, diameter, fraction(gauge.percent));
        Item::new(gauge.tone, self.pulse)
            .with(RingTrack(ring), TRACK)
            .with(RingArc(ring), self.tints.tone(gauge.tone).rgba(1.0))
    }

    fn rings(&self, gauges: &[Gauge]) -> Vec<Item> {
        let size = self.size;
        match gauges {
            [single] => vec![self.ring(size / 2.0, size, *single)],
            [first, second, ..] => {
                let diameter = (size - size * RING_GAP_RATIO) / 2.0;
                vec![
                    self.ring(diameter / 2.0, diameter, *first),
                    self.ring(size - diameter / 2.0, diameter, *second),
                ]
            }
            [] => Vec::new(),
        }
    }

    fn pill(&self, center_y: f64, width: f64) -> RoundedRect {
        let height = (self.size * BAR_HEIGHT_RATIO).max(MIN_BAR_HEIGHT);
        RoundedRect {
            x: 0.0,
            y: center_y - height / 2.0,
            width,
            height,
            radius: height / 2.0,
        }
    }

    fn tick(&self, center_y: f64, position: u8) -> RoundedRect {
        let width = (self.size * TICK_WIDTH_RATIO).max(MIN_TICK_WIDTH);
        let height = self.size * TICK_HEIGHT_RATIO;
        RoundedRect {
            x: (self.size * fraction(position) - width / 2.0)
                .min(self.size - width)
                .max(0.0),
            y: center_y - height / 2.0,
            width,
            height,
            radius: width / 2.0,
        }
    }

    fn bar(&self, center_y: f64, gauge: Gauge) -> Item {
        let track = self.pill(center_y, self.size);
        let mut item = Item::new(gauge.tone, self.pulse).with(track, TRACK);
        if gauge.percent > 0 {
            let width = (self.size * fraction(gauge.percent))
                .max(track.height)
                .min(self.size);
            item = item.with(
                self.pill(center_y, width),
                self.tints.tone(gauge.tone).rgba(1.0),
            );
        }
        match gauge.tick {
            Some(position) => item.with(self.tick(center_y, position), self.tints.tick.rgba(1.0)),
            None => item,
        }
    }

    fn bars(&self, gauges: &[Gauge]) -> Vec<Item> {
        let middle = self.size / 2.0;
        let offset = self.size * BAR_OFFSET_RATIO;
        match gauges {
            [single] => vec![self.bar(middle, *single)],
            [first, second, ..] => vec![
                self.bar(middle - offset, *first),
                self.bar(middle + offset, *second),
            ],
            [] => Vec::new(),
        }
    }

    fn mark(&self, plates: &[Polygon], tone: Tone) -> Vec<Item> {
        let color = self.tints.tone(tone).rgba(1.0);
        let item = plates
            .iter()
            .fold(Item::new(tone, self.pulse), |item, plate| {
                item.with(plate.scaled(self.size / MARK_GRID), color)
            });
        vec![item]
    }

    fn items(&self, glyph: &Glyph, plates: &[Polygon]) -> Option<Vec<Item>> {
        match glyph {
            Glyph::Mark(None) => None,
            Glyph::Mark(Some(tone)) => Some(self.mark(plates, *tone)),
            Glyph::Rings(gauges) => Some(self.rings(gauges)),
            Glyph::Bars(gauges) => Some(self.bars(gauges)),
        }
    }
}

#[must_use]
pub fn render_at(
    glyph: &Glyph,
    size: u32,
    plates: &[Polygon],
    tints: &Tints,
    pulse: u8,
) -> Option<Pixmap> {
    let frame = Frame {
        size: f64::from(size),
        tints,
        pulse,
    };
    let items = frame.items(glyph, plates)?;
    let groups: Vec<Group> = items
        .iter()
        .map(|item| Group {
            layers: item
                .shapes
                .iter()
                .map(|(shape, color)| Layer {
                    shape: shape.as_ref(),
                    color: *color,
                })
                .collect(),
            opacity: item.opacity,
        })
        .collect();
    Some(rasterize(size, &groups))
}

#[must_use]
pub fn render(glyph: &Glyph, plates: &[Polygon], tints: &Tints, pulse: u8) -> Option<Vec<Pixmap>> {
    SIZES
        .iter()
        .map(|size| render_at(glyph, *size, plates, tints, pulse))
        .collect()
}

#[cfg(test)]
#[path = "layout_tests.rs"]
mod tests;
