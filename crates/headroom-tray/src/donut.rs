use std::f64::consts::{FRAC_PI_2, PI, TAU};

const HOLE_RATIO: f64 = 0.618;
const GAP_RATIO: f64 = 2.0 / 104.0;
const CORNER_RATIO: f64 = 0.15;
const MIN_WIDTH_RATIO: f64 = 3.0 / 104.0;
const ANGLE_EPSILON: f64 = 1e-9;
pub const START_ANGLE: f64 = -FRAC_PI_2;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Geometry {
    pub center: f64,
    pub outer: f64,
    pub inner: f64,
    pub gap: f64,
    pub corner: f64,
}

#[must_use]
pub fn geometry(size: f64) -> Geometry {
    let outer = size / 2.0;
    let thickness = outer * (1.0 - HOLE_RATIO);
    Geometry {
        center: outer,
        outer,
        inner: outer - thickness,
        gap: GAP_RATIO * size,
        corner: CORNER_RATIO * thickness,
    }
}

#[must_use]
pub fn min_fraction() -> f64 {
    let unit = geometry(1.0);
    (MIN_WIDTH_RATIO + unit.gap) / unit.inner / TAU
}

fn shares(values: &[f64], raised: &[bool], minimum: f64) -> Vec<f64> {
    let free_total: f64 = values
        .iter()
        .zip(raised)
        .filter(|(_, raised)| !**raised)
        .map(|(value, _)| value)
        .sum();
    let raised_count = raised.iter().filter(|raised| **raised).count();
    #[allow(clippy::cast_precision_loss, reason = "slice counts are tiny")]
    let share = 1.0 - raised_count as f64 * minimum;
    values
        .iter()
        .zip(raised)
        .map(|(value, raised)| {
            if *raised {
                minimum
            } else {
                value / free_total * share
            }
        })
        .collect()
}

#[must_use]
pub fn visible_fractions(values: &[f64]) -> Vec<f64> {
    let total: f64 = values.iter().sum();
    if total <= 0.0 {
        return vec![0.0; values.len()];
    }
    #[allow(clippy::cast_precision_loss, reason = "slice counts are tiny")]
    let minimum = if values.len() > 1 {
        (1.0 / values.len() as f64).min(min_fraction())
    } else {
        0.0
    };
    let mut raised = vec![false; values.len()];
    let mut fractions = shares(values, &raised, minimum);
    while let Some(index) = (0..values.len()).find(|&i| !raised[i] && fractions[i] < minimum) {
        raised[index] = true;
        fractions = shares(values, &raised, minimum);
    }
    fractions
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Segment {
    pub index: usize,
    pub start: f64,
    pub end: f64,
    pub gap: bool,
}

#[must_use]
pub fn segments(fractions: &[f64]) -> Vec<Segment> {
    if let [only] = fractions {
        let end = START_ANGLE + only.min(1.0) * TAU;
        return if end > START_ANGLE {
            vec![Segment {
                index: 0,
                start: START_ANGLE,
                end,
                gap: false,
            }]
        } else {
            Vec::new()
        };
    }
    let mut angle = START_ANGLE;
    let mut out = Vec::new();
    for (index, fraction) in fractions.iter().enumerate() {
        let start = angle;
        angle += fraction * TAU;
        if angle > start {
            out.push(Segment {
                index,
                start,
                end: angle,
                gap: true,
            });
        }
    }
    out
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Arc {
    pub center: (f64, f64),
    pub radius: f64,
    pub from: f64,
    pub to: f64,
    pub negative: bool,
    pub new_sub_path: bool,
}

fn arc(center: (f64, f64), radius: f64, from: f64, to: f64) -> Arc {
    Arc {
        center,
        radius,
        from,
        to,
        negative: false,
        new_sub_path: false,
    }
}

fn corner_radius(geometry: &Geometry, half_sweep: f64, half_gap: f64) -> Option<f64> {
    let sin = half_sweep.min(FRAC_PI_2).sin();
    let inner_room = (geometry.inner * sin - half_gap) / (1.0 - sin);
    let outer_room = (geometry.outer * sin - half_gap) / (1.0 + sin);
    let room = inner_room.min(outer_room);
    (room >= 0.0).then(|| geometry.corner.min(room))
}

fn polar(geometry: &Geometry, radius: f64, angle: f64) -> (f64, f64) {
    (
        geometry.center + radius * angle.cos(),
        geometry.center + radius * angle.sin(),
    )
}

fn ring_path(geometry: &Geometry) -> Vec<Arc> {
    let center = (geometry.center, geometry.center);
    vec![
        arc(center, geometry.outer, 0.0, TAU),
        Arc {
            negative: true,
            new_sub_path: true,
            ..arc(center, geometry.inner, TAU, 0.0)
        },
    ]
}

struct Offsets {
    corner: f64,
    outer: f64,
    inner: f64,
}

fn outer_arcs(geometry: &Geometry, segment: &Segment, offsets: &Offsets) -> [Arc; 3] {
    let (start, end, corner, outer) = (segment.start, segment.end, offsets.corner, offsets.outer);
    let center = (geometry.center, geometry.center);
    let rounded = |angle| polar(geometry, geometry.outer - corner, angle);
    [
        arc(
            rounded(start + outer),
            corner,
            start - FRAC_PI_2,
            start + outer,
        ),
        arc(
            center,
            geometry.outer,
            start + outer,
            (end - outer).max(start + outer),
        ),
        arc(rounded(end - outer), corner, end - outer, end + FRAC_PI_2),
    ]
}

fn inner_arcs(geometry: &Geometry, segment: &Segment, offsets: &Offsets) -> [Arc; 3] {
    let (start, end, corner, inner) = (segment.start, segment.end, offsets.corner, offsets.inner);
    let center = (geometry.center, geometry.center);
    let rounded = |angle| polar(geometry, geometry.inner + corner, angle);
    let back = (start + inner).min(end - inner);
    [
        arc(
            rounded(end - inner),
            corner,
            end + FRAC_PI_2,
            end - inner + PI,
        ),
        Arc {
            negative: true,
            ..arc(center, geometry.inner, end - inner, back)
        },
        arc(
            rounded(start + inner),
            corner,
            start + inner + PI,
            start + 3.0 * FRAC_PI_2,
        ),
    ]
}

#[must_use]
pub fn sector_path(geometry: &Geometry, segment: &Segment) -> Option<Vec<Arc>> {
    if !segment.gap && segment.end - segment.start >= TAU - ANGLE_EPSILON {
        return Some(ring_path(geometry));
    }
    let half_gap = if segment.gap { geometry.gap / 2.0 } else { 0.0 };
    let corner = corner_radius(geometry, (segment.end - segment.start) / 2.0, half_gap)?;
    let outer = ((half_gap + corner) / (geometry.outer - corner)).asin();
    let inner = ((half_gap + corner) / (geometry.inner + corner)).asin();
    let offsets = Offsets {
        corner,
        outer,
        inner,
    };
    let outer = outer_arcs(geometry, segment, &offsets);
    let inner = inner_arcs(geometry, segment, &offsets);
    Some(outer.into_iter().chain(inner).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    #[test]
    fn geometry_follows_the_ratios() {
        let g = geometry(104.0);
        assert!(close(g.outer, 52.0));
        assert!(close(g.inner, 52.0 * HOLE_RATIO));
        assert!(close(g.gap, 2.0));
    }

    #[test]
    fn fractions_sum_to_one_and_raise_tiny_slices() {
        let fractions = visible_fractions(&[1000.0, 1.0, 500.0]);
        assert!(close(fractions.iter().sum(), 1.0));
        assert!(close(fractions[1], min_fraction()));
        assert!(fractions[0] > fractions[2]);
        assert_eq!(visible_fractions(&[0.0, 0.0]), vec![0.0, 0.0]);
        assert_eq!(visible_fractions(&[5.0]), vec![1.0]);
    }

    #[test]
    fn single_provider_is_a_full_ring() {
        let segs = segments(&[1.0]);
        assert_eq!(segs.len(), 1);
        assert!(!segs[0].gap);
        let path = sector_path(&geometry(104.0), &segs[0]).unwrap();
        assert_eq!(path.len(), 2);
        assert!(path[1].new_sub_path);
    }

    #[test]
    fn slices_run_clockwise_from_twelve() {
        let segs = segments(&[0.5, 0.5]);
        assert!(close(segs[0].start, START_ANGLE));
        assert!(close(segs[1].end, START_ANGLE + TAU));
        let path = sector_path(&geometry(104.0), &segs[0]).unwrap();
        assert_eq!(path.len(), 6);
        assert!(path[4].negative);
    }

    #[test]
    fn slices_without_room_are_skipped() {
        let tiny = Segment {
            index: 0,
            start: 0.0,
            end: 1e-6,
            gap: true,
        };
        assert!(sector_path(&geometry(104.0), &tiny).is_none());
    }
}
