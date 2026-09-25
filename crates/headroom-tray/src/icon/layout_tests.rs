use super::*;
use crate::icon::canvas::to_byte;
use crate::icon::mark::parse_mark;
use crate::icon::tints::Color;

const RED: Color = Color([255, 0, 0, 255]);
const AMBER: Color = Color([255, 204, 0, 255]);
const BLUE: Color = Color([0, 136, 255, 255]);
const TICK: Color = Color([255, 255, 255, 255]);

fn tints() -> Tints {
    Tints {
        good: BLUE,
        warning: AMBER,
        critical: RED,
        neutral: Color([128, 128, 128, 255]),
        tick: TICK,
    }
}

fn plates() -> Vec<Polygon> {
    parse_mark(crate::assets::MARK).unwrap()
}

fn gauge(percent: u8, tick: Option<u8>, tone: Tone) -> Gauge {
    Gauge {
        percent,
        tick,
        tone,
    }
}

fn pixel(pixmap: &Pixmap, x: u32, y: u32) -> [u8; 4] {
    let offset = ((y * pixmap.size + x) * 4) as usize;
    pixmap.argb[offset..offset + 4].try_into().unwrap()
}

fn draw(glyph: &Glyph, size: u32) -> Pixmap {
    render_at(glyph, size, &plates(), &tints(), FULL_OPACITY).unwrap()
}

#[test]
fn every_size_is_rendered_as_argb() {
    let glyph = Glyph::Rings(vec![gauge(50, None, Tone::Critical)]);
    let pixmaps = render(&glyph, &plates(), &tints(), FULL_OPACITY).unwrap();
    assert_eq!(pixmaps.len(), SIZES.len());
    for pixmap in pixmaps {
        assert_eq!(pixmap.argb.len(), (pixmap.size * pixmap.size * 4) as usize);
    }
}

#[test]
fn the_themed_mark_is_left_to_the_host() {
    assert!(render(&Glyph::Mark(None), &plates(), &tints(), FULL_OPACITY).is_none());
}

#[test]
fn a_single_ring_matches_the_classic_icon() {
    let half = draw(&Glyph::Rings(vec![gauge(50, None, Tone::Critical)]), 32);
    assert_eq!(pixel(&half, 29, 16), [255, 255, 0, 0]);
    let left = pixel(&half, 2, 16);
    assert_eq!(left[0], to_byte(0.45));
    assert_eq!(left[1], left[2]);
    assert_eq!(pixel(&half, 16, 16), [0; 4]);
    let empty = draw(&Glyph::Rings(vec![gauge(0, None, Tone::Critical)]), 22);
    assert!(empty.argb.chunks(4).all(|p| p[1] == p[2]));
    let full = draw(&Glyph::Rings(vec![gauge(100, None, Tone::Critical)]), 22);
    assert_eq!(pixel(&full, 2, 11), [255, 255, 0, 0]);
}

#[test]
fn two_rings_sit_side_by_side_in_their_tones() {
    let glyph = Glyph::Rings(vec![
        gauge(100, None, Tone::Good),
        gauge(100, None, Tone::Warning),
    ]);
    let pixmap = draw(&glyph, 32);
    assert_eq!(pixel(&pixmap, 1, 16), [255, 0, 136, 255]);
    assert_eq!(pixel(&pixmap, 30, 16), [255, 255, 204, 0]);
    assert_eq!(pixel(&pixmap, 16, 16), [0; 4]);
    assert_eq!(pixel(&pixmap, 16, 2), [0; 4]);
}

#[test]
fn a_bar_fills_from_the_leading_edge_with_a_tick() {
    let pixmap = draw(&Glyph::Bars(vec![gauge(50, Some(75), Tone::Warning)]), 26);
    assert_eq!(pixel(&pixmap, 4, 13), [255, 255, 204, 0]);
    assert_eq!(pixel(&pixmap, 16, 13)[0], to_byte(0.45));
    assert_eq!(pixel(&pixmap, 19, 10), [255, 255, 255, 255]);
    assert_eq!(pixel(&pixmap, 13, 4), [0; 4]);
}

#[test]
fn two_bars_stack_and_hide_missing_ticks() {
    let glyph = Glyph::Bars(vec![
        gauge(100, None, Tone::Good),
        gauge(0, None, Tone::Good),
    ]);
    let pixmap = draw(&glyph, 32);
    assert_eq!(pixel(&pixmap, 16, 8), [255, 0, 136, 255]);
    assert_eq!(pixel(&pixmap, 16, 24)[0], to_byte(0.45));
    assert_eq!(pixel(&pixmap, 16, 16), [0; 4]);
}

#[test]
fn a_tinted_mark_draws_the_brand_plates() {
    let pixmap = draw(&Glyph::Mark(Some(Tone::Warning)), 16);
    assert_eq!(pixel(&pixmap, 3, 9), [255, 255, 204, 0]);
    assert_eq!(pixel(&pixmap, 0, 0), [0; 4]);
}

#[test]
fn only_critical_items_fade_with_the_pulse() {
    let glyph = Glyph::Rings(vec![
        gauge(100, None, Tone::Good),
        gauge(100, None, Tone::Critical),
    ]);
    let dim = render_at(&glyph, 32, &plates(), &tints(), 140).unwrap();
    assert_eq!(pixel(&dim, 1, 16)[0], 255);
    assert_eq!(pixel(&dim, 30, 16)[0], 140);
}
