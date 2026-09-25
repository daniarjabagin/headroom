use std::f64::consts::TAU;

use super::canvas::Shape;

const LINE_RATIO: f64 = 0.2;
const MIN_LINE: f64 = 2.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ring {
    center_x: f64,
    center_y: f64,
    radius: f64,
    half_line: f64,
    sweep: f64,
}

impl Ring {
    #[must_use]
    pub fn new(center_x: f64, center_y: f64, diameter: f64, fraction: f64) -> Self {
        let line = (diameter * LINE_RATIO).max(MIN_LINE);
        Self {
            center_x,
            center_y,
            radius: (diameter - line) / 2.0,
            half_line: line / 2.0,
            sweep: fraction.clamp(0.0, 1.0) * TAU,
        }
    }

    fn on_track(&self, x: f64, y: f64) -> bool {
        let distance = (x - self.center_x).hypot(y - self.center_y);
        (distance - self.radius).abs() <= self.half_line
    }

    fn on_arc(&self, x: f64, y: f64) -> bool {
        if self.sweep <= 0.0 {
            return false;
        }
        let angle = (x - self.center_x).atan2(self.center_y - y).rem_euclid(TAU);
        let within = self.on_track(x, y) && (angle <= self.sweep || self.sweep >= TAU);
        within || self.near_cap(x, y, 0.0) || self.near_cap(x, y, self.sweep)
    }

    fn near_cap(&self, x: f64, y: f64, angle: f64) -> bool {
        let cap_x = self.center_x + self.radius * angle.sin();
        let cap_y = self.center_y - self.radius * angle.cos();
        (x - cap_x).hypot(y - cap_y) <= self.half_line
    }
}

pub struct RingTrack(pub Ring);

impl Shape for RingTrack {
    fn contains(&self, x: f64, y: f64) -> bool {
        self.0.on_track(x, y)
    }
}

pub struct RingArc(pub Ring);

impl Shape for RingArc {
    fn contains(&self, x: f64, y: f64) -> bool {
        self.0.on_arc(x, y)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RoundedRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub radius: f64,
}

impl Shape for RoundedRect {
    fn contains(&self, x: f64, y: f64) -> bool {
        let inside =
            x >= self.x && x <= self.x + self.width && y >= self.y && y <= self.y + self.height;
        if !inside {
            return false;
        }
        let radius = self.radius.min(self.width / 2.0).min(self.height / 2.0);
        let nearest_x = x.min(self.x + self.width - radius).max(self.x + radius);
        let nearest_y = y.min(self.y + self.height - radius).max(self.y + radius);
        (x - nearest_x).hypot(y - nearest_y) <= radius
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Polygon {
    pub points: Vec<(f64, f64)>,
}

impl Polygon {
    #[must_use]
    pub fn scaled(&self, factor: f64) -> Self {
        Self {
            points: self
                .points
                .iter()
                .map(|(x, y)| (x * factor, y * factor))
                .collect(),
        }
    }
}

impl Shape for Polygon {
    fn contains(&self, x: f64, y: f64) -> bool {
        let count = self.points.len();
        let mut inside = false;
        for index in 0..count {
            let (ax, ay) = self.points[index];
            let (bx, by) = self.points[(index + count - 1) % count];
            if (ay > y) != (by > y) && x < (bx - ax) * (y - ay) / (by - ay) + ax {
                inside = !inside;
            }
        }
        inside
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn half_ring_arc_covers_the_right_side() {
        let ring = Ring::new(16.0, 16.0, 32.0, 0.5);
        assert!(RingArc(ring).contains(29.5, 16.5));
        assert!(!RingArc(ring).contains(2.5, 16.5));
        assert!(RingTrack(ring).contains(2.5, 16.5));
        assert!(!RingTrack(ring).contains(16.0, 16.0));
    }

    #[test]
    fn rounded_rect_cuts_its_corners() {
        let pill = RoundedRect {
            x: 0.0,
            y: 0.0,
            width: 10.0,
            height: 4.0,
            radius: 2.0,
        };
        assert!(pill.contains(5.0, 2.0));
        assert!(!pill.contains(0.1, 0.1));
        assert!(pill.contains(0.2, 2.0));
        assert!(!pill.contains(11.0, 2.0));
    }

    #[test]
    fn circle_sized_rects_survive_rounding_noise() {
        for size in 10..80u32 {
            let side = f64::from(size) * 5.0 / 26.0;
            let center = f64::from(size) * 0.75;
            let dot = RoundedRect {
                x: 0.0,
                y: center - side / 2.0,
                width: side,
                height: side,
                radius: side / 2.0,
            };
            assert!(dot.contains(side / 2.0, center));
        }
    }

    #[test]
    fn polygon_uses_the_even_odd_rule() {
        let square = Polygon {
            points: vec![(0.0, 0.0), (4.0, 0.0), (4.0, 4.0), (0.0, 4.0)],
        };
        assert!(square.contains(2.0, 2.0));
        assert!(!square.contains(5.0, 2.0));
        assert!(square.scaled(2.0).contains(7.0, 7.0));
    }
}
