const MIN_SCALE: f64 = 0.5;
const STEPS: u32 = 24;
pub const STROKE_CLEARANCE: f64 = 3.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LineMetrics {
    pub width: f64,
    pub height: f64,
    pub ink_top: f64,
    pub ink_bottom: f64,
}

impl LineMetrics {
    fn scaled(self, scale: f64) -> Self {
        Self {
            width: self.width * scale,
            height: self.height * scale,
            ink_top: self.ink_top * scale,
            ink_bottom: self.ink_bottom * scale,
        }
    }
}

fn fits(radius: f64, lines: &[LineMetrics], scales: &[f64]) -> bool {
    let sized: Vec<LineMetrics> = lines
        .iter()
        .zip(scales)
        .map(|(line, scale)| line.scaled(*scale))
        .collect();
    let total: f64 = sized.iter().map(|line| line.height).sum();
    let mut top = -total / 2.0;
    sized.iter().all(|line| {
        let ink_top = top + line.ink_top.clamp(0.0, line.height);
        let ink_bottom = top + line.ink_bottom.clamp(0.0, line.height);
        top += line.height;
        let reach = ink_top.abs().max(ink_bottom.abs());
        (line.width.max(0.0) / 2.0).hypot(reach) <= radius
    })
}

fn largest(accepts: impl Fn(f64) -> bool) -> Option<f64> {
    if !accepts(MIN_SCALE) {
        return None;
    }
    let (mut low, mut high) = (MIN_SCALE, 1.0);
    for _ in 0..STEPS {
        let middle = f64::midpoint(low, high);
        if accepts(middle) {
            low = middle;
        } else {
            high = middle;
        }
    }
    Some(low)
}

fn with_lead(lead: f64, rest: f64, count: usize) -> Vec<f64> {
    std::iter::once(lead)
        .chain(std::iter::repeat(rest))
        .take(count)
        .collect()
}

#[must_use]
pub fn text_radius(inner_radius: f64) -> f64 {
    (inner_radius - STROKE_CLEARANCE).max(0.0)
}

#[must_use]
pub fn fit_scales(radius: f64, lines: &[LineMetrics]) -> Vec<f64> {
    let count = lines.len();
    if fits(radius, lines, &with_lead(1.0, 1.0, count)) {
        return with_lead(1.0, 1.0, count);
    }
    if let Some(lead) = largest(|lead| fits(radius, lines, &with_lead(lead, 1.0, count))) {
        return with_lead(lead, 1.0, count);
    }
    let rest = largest(|rest| fits(radius, lines, &with_lead(MIN_SCALE, rest, count)));
    with_lead(MIN_SCALE, rest.unwrap_or(MIN_SCALE), count)
}

#[cfg(test)]
mod tests {
    use super::*;

    const RADIUS: f64 = 29.0;

    fn line(width: f64, height: f64) -> LineMetrics {
        LineMetrics {
            width,
            height,
            ink_top: height * 0.2,
            ink_bottom: height * 0.8,
        }
    }

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-6
    }

    #[test]
    fn short_text_keeps_its_size() {
        assert_eq!(fit_scales(RADIUS, &[line(40.0, 18.0)]), [1.0]);
        assert_eq!(
            fit_scales(RADIUS, &[line(40.0, 18.0), line(30.0, 11.0)]),
            [1.0, 1.0]
        );
        assert!(fit_scales(RADIUS, &[]).is_empty());
    }

    #[test]
    fn a_wide_value_shrinks_alone_until_it_clears_the_stroke() {
        let cases = [
            vec![line(78.0, 18.0)],
            vec![line(70.0, 18.0), line(34.0, 11.0)],
            vec![line(64.0, 18.0), line(20.0, 11.0)],
        ];
        for lines in cases {
            let scales = fit_scales(RADIUS, &lines);
            assert!(scales[0] < 1.0, "{lines:?}");
            assert!(
                scales[1..].iter().all(|scale| close(*scale, 1.0)),
                "{lines:?}"
            );
            assert!(fits(RADIUS, &lines, &scales), "{lines:?} at {scales:?}");
            let mut bigger = scales.clone();
            bigger[0] += 0.01;
            assert!(!fits(RADIUS, &lines, &bigger), "{lines:?} at {scales:?}");
        }
    }

    #[test]
    fn the_unit_line_shrinks_only_when_the_value_is_at_its_floor() {
        let lines = [line(400.0, 18.0), line(60.0, 11.0)];
        let scales = fit_scales(RADIUS, &lines);
        assert!(close(scales[0], MIN_SCALE));
        assert!(close(scales[1], MIN_SCALE));
        let lines = [line(100.0, 18.0), line(56.0, 11.0)];
        let scales = fit_scales(RADIUS, &lines);
        assert!(close(scales[0], MIN_SCALE));
        assert!(scales[1] > MIN_SCALE && scales[1] < 1.0);
        assert!(fits(RADIUS, &lines, &scales));
    }

    #[test]
    fn the_second_line_counts_for_the_height() {
        let alone = fit_scales(RADIUS, &[line(54.0, 18.0)])[0];
        let stacked = fit_scales(RADIUS, &[line(54.0, 18.0), line(20.0, 11.0)])[0];
        assert!(stacked < alone);
    }

    #[test]
    fn scale_never_drops_below_the_floor() {
        assert_eq!(fit_scales(RADIUS, &[line(400.0, 18.0)]), [MIN_SCALE]);
        assert_eq!(fit_scales(0.0, &[line(10.0, 10.0)]), [MIN_SCALE]);
    }

    #[test]
    fn text_radius_leaves_room_for_the_stroke() {
        assert!(close(text_radius(32.0), 29.0));
        assert!(close(text_radius(1.0), 0.0));
    }
}
