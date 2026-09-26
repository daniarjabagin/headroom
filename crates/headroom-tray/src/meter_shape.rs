pub const METER_HEIGHT: i32 = 9;
pub const TRACK_HEIGHT: f64 = 5.0;
pub const TICK_WIDTH: f64 = 2.0;
pub const TICK_RADIUS: f64 = 1.0;
pub const SEGMENT_GAP: f64 = 2.0;
pub const SHEEN_MIN_FRACTION: f64 = 0.03;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Slot {
    pub x: f64,
    pub width: f64,
}

#[must_use]
pub fn track_top(height: f64) -> f64 {
    ((height - TRACK_HEIGHT) / 2.0).round()
}

#[must_use]
pub fn fill_width(width: f64, fraction: f64) -> f64 {
    if fraction <= 0.0 {
        return 0.0;
    }
    (width * fraction).round().max(TRACK_HEIGHT).min(width)
}

#[must_use]
pub fn shows_sheen(fraction: f64) -> bool {
    fraction >= SHEEN_MIN_FRACTION
}

#[must_use]
pub fn sheen_rect(width: f64, height: f64, track: f64, fraction: f64) -> Option<Rect> {
    shows_sheen(fraction).then(|| Rect {
        x: 0.0,
        y: ((height - track) / 2.0).round(),
        width: fill_width(width, fraction),
        height: track,
    })
}

#[must_use]
pub fn tick_rect(slot: Slot, height: f64, tick: f64) -> Rect {
    let offset = (slot.width * tick - TICK_WIDTH / 2.0)
        .round()
        .min(slot.width - TICK_WIDTH)
        .max(0.0);
    Rect {
        x: slot.x + offset,
        y: 0.0,
        width: TICK_WIDTH,
        height,
    }
}

#[must_use]
pub fn segment_layout(count: usize, width: f64, gap: f64) -> Vec<Slot> {
    let Ok(parts) = u32::try_from(count) else {
        return Vec::new();
    };
    if parts == 0 {
        return Vec::new();
    }
    let total = f64::from(parts);
    let usable = (width - gap * (total - 1.0)).max(0.0);
    let base = (usable / total).floor();
    let extra = usable - base * total;
    let mut x = 0.0;
    (0..parts)
        .map(|index| {
            let wider = f64::from(index) < extra;
            let slot = Slot {
                x,
                width: base + if wider { 1.0 } else { 0.0 },
            };
            x += slot.width + gap;
            slot
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const WHOLE: Slot = Slot {
        x: 0.0,
        width: 200.0,
    };

    #[test]
    fn tick_is_a_full_height_bar_taller_than_the_track() {
        let height = f64::from(METER_HEIGHT);
        let rect = tick_rect(WHOLE, height, 0.4);
        assert_eq!(
            rect,
            Rect {
                x: 79.0,
                y: 0.0,
                width: TICK_WIDTH,
                height,
            }
        );
        assert!(rect.height > TRACK_HEIGHT);
        assert!(rect.width >= 2.0 * TICK_RADIUS);
    }

    #[test]
    fn tick_stays_inside_its_slot() {
        let slot = Slot {
            x: 50.0,
            width: 40.0,
        };
        assert!((tick_rect(slot, 9.0, 0.0).x - 50.0).abs() < 1e-9);
        assert!((tick_rect(slot, 9.0, 1.0).x - 88.0).abs() < 1e-9);
        assert!((tick_rect(slot, 9.0, 0.5).x - 69.0).abs() < 1e-9);
    }

    #[test]
    fn track_sits_in_the_middle_of_the_meter() {
        assert!((track_top(9.0) - 2.0).abs() < 1e-9);
    }

    #[test]
    fn fill_keeps_a_round_minimum_and_never_overflows() {
        assert!(fill_width(200.0, 0.0).abs() < 1e-9);
        assert!((fill_width(200.0, 0.001) - TRACK_HEIGHT).abs() < 1e-9);
        assert!((fill_width(200.0, 0.5) - 100.0).abs() < 1e-9);
        assert!((fill_width(200.0, 1.5) - 200.0).abs() < 1e-9);
    }

    #[test]
    fn sheen_covers_the_fill_and_skips_slivers() {
        assert_eq!(
            sheen_rect(200.0, 9.0, TRACK_HEIGHT, 0.5),
            Some(Rect {
                x: 0.0,
                y: 2.0,
                width: 100.0,
                height: TRACK_HEIGHT,
            })
        );
        assert_eq!(
            sheen_rect(200.0, 8.0, 4.0, 1.2).map(|rect| (rect.y, rect.width)),
            Some((2.0, 200.0))
        );
        assert_eq!(sheen_rect(200.0, 9.0, TRACK_HEIGHT, 0.029), None);
        assert!(sheen_rect(200.0, 9.0, TRACK_HEIGHT, SHEEN_MIN_FRACTION).is_some());
        assert!(!shows_sheen(0.0));
        assert!(shows_sheen(0.03));
    }

    #[test]
    fn segments_split_pixels_like_gnome() {
        let slots = segment_layout(3, 101.0, SEGMENT_GAP);
        let widths: Vec<f64> = slots.iter().map(|slot| slot.width).collect();
        let starts: Vec<f64> = slots.iter().map(|slot| slot.x).collect();
        assert_eq!(widths, [33.0, 32.0, 32.0]);
        assert_eq!(starts, [0.0, 35.0, 69.0]);
        assert!(segment_layout(0, 100.0, SEGMENT_GAP).is_empty());
    }
}
