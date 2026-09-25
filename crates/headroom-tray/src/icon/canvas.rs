use super::Pixmap;
use crate::palette::Rgba;

const SAMPLES: u32 = 4;

pub trait Shape {
    fn contains(&self, x: f64, y: f64) -> bool;
}

pub struct Layer<'a> {
    pub shape: &'a dyn Shape,
    pub color: Rgba,
}

pub struct Group<'a> {
    pub layers: Vec<Layer<'a>>,
    pub opacity: f64,
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the value is clamped to 0..=255 first"
)]
#[must_use]
pub fn to_byte(value: f64) -> u8 {
    (value.clamp(0.0, 1.0) * 255.0).round() as u8
}

fn over(top: [f64; 4], below: [f64; 4]) -> [f64; 4] {
    let keep = 1.0 - top[3];
    [
        top[0] + below[0] * keep,
        top[1] + below[1] * keep,
        top[2] + below[2] * keep,
        top[3] + below[3] * keep,
    ]
}

fn premultiplied(color: Rgba) -> [f64; 4] {
    [
        color.red * color.alpha,
        color.green * color.alpha,
        color.blue * color.alpha,
        color.alpha,
    ]
}

fn composite_group(group: &Group, x: f64, y: f64) -> [f64; 4] {
    let flat = group
        .layers
        .iter()
        .filter(|layer| layer.shape.contains(x, y))
        .fold([0.0; 4], |below, layer| {
            over(premultiplied(layer.color), below)
        });
    flat.map(|channel| channel * group.opacity)
}

fn composite(groups: &[Group], x: f64, y: f64) -> [f64; 4] {
    groups.iter().fold([0.0; 4], |below, group| {
        over(composite_group(group, x, y), below)
    })
}

fn pixel(groups: &[Group], px: u32, py: u32) -> [u8; 4] {
    let step = 1.0 / f64::from(SAMPLES);
    let mut sum = [0.0; 4];
    for sy in 0..SAMPLES {
        for sx in 0..SAMPLES {
            let x = f64::from(px) + (f64::from(sx) + 0.5) * step;
            let y = f64::from(py) + (f64::from(sy) + 0.5) * step;
            for (total, value) in sum.iter_mut().zip(composite(groups, x, y)) {
                *total += value;
            }
        }
    }
    let count = f64::from(SAMPLES * SAMPLES);
    let alpha = sum[3] / count;
    if alpha <= 0.0 {
        return [0; 4];
    }
    let channel = |premultiplied: f64| to_byte(premultiplied / count / alpha);
    [
        to_byte(alpha),
        channel(sum[0]),
        channel(sum[1]),
        channel(sum[2]),
    ]
}

#[must_use]
pub fn rasterize(size: u32, groups: &[Group]) -> Pixmap {
    let mut argb = Vec::with_capacity((size * size * 4) as usize);
    for y in 0..size {
        for x in 0..size {
            argb.extend_from_slice(&pixel(groups, x, y));
        }
    }
    Pixmap { size, argb }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct LeftHalf;

    impl Shape for LeftHalf {
        fn contains(&self, x: f64, _y: f64) -> bool {
            x < 1.0
        }
    }

    struct Everything;

    impl Shape for Everything {
        fn contains(&self, _x: f64, _y: f64) -> bool {
            true
        }
    }

    const RED: Rgba = Rgba {
        red: 1.0,
        green: 0.0,
        blue: 0.0,
        alpha: 1.0,
    };

    const GRAY: Rgba = Rgba {
        red: 0.5,
        green: 0.5,
        blue: 0.5,
        alpha: 0.5,
    };

    fn group(opacity: f64) -> Group<'static> {
        Group {
            layers: vec![
                Layer {
                    shape: &Everything,
                    color: GRAY,
                },
                Layer {
                    shape: &LeftHalf,
                    color: RED,
                },
            ],
            opacity,
        }
    }

    #[test]
    fn upper_layers_cover_lower_ones_per_sample() {
        let pixmap = rasterize(2, &[group(1.0)]);
        assert_eq!(&pixmap.argb[0..4], &[255, 255, 0, 0]);
        assert_eq!(&pixmap.argb[4..8], &[128, 128, 128, 128]);
    }

    #[test]
    fn group_opacity_fades_the_flattened_group() {
        let pixmap = rasterize(2, &[group(0.5)]);
        assert_eq!(&pixmap.argb[0..4], &[128, 255, 0, 0]);
    }

    #[test]
    fn empty_canvas_is_transparent() {
        let pixmap = rasterize(3, &[]);
        assert_eq!(pixmap.argb, vec![0; 36]);
    }
}
