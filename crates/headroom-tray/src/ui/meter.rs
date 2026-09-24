use std::cell::Cell;
use std::rc::Rc;

use gtk::cairo;
use gtk::prelude::*;

use crate::palette::Rgba;
use crate::ui::draw::{capsule, fill, set_color};
use crate::ui::motion;

const METER_HEIGHT: i32 = 9;
const TRACK_HEIGHT: f64 = 5.0;
const TICK_WIDTH: f64 = 2.0;

#[derive(Debug, Clone, Copy)]
pub struct MeterColors {
    pub track: Rgba,
    pub fill: Rgba,
    pub tick: Rgba,
}

#[derive(Debug, Clone, Copy)]
struct MeterShape {
    fraction: f64,
    tick: Option<f64>,
}

fn fill_width(width: f64, fraction: f64) -> f64 {
    if fraction <= 0.0 {
        return 0.0;
    }
    (width * fraction).round().max(TRACK_HEIGHT).min(width)
}

fn draw(cr: &cairo::Context, width: f64, height: f64, shape: MeterShape, colors: MeterColors) {
    let top = ((height - TRACK_HEIGHT) / 2.0).round();
    set_color(cr, colors.track);
    capsule(cr, 0.0, top, width, TRACK_HEIGHT);
    fill(cr);
    let filled = fill_width(width, shape.fraction);
    if filled > 0.0 {
        set_color(cr, colors.fill);
        capsule(cr, 0.0, top, filled, TRACK_HEIGHT);
        fill(cr);
    }
    if let Some(tick) = shape.tick {
        let x = (width * tick - TICK_WIDTH / 2.0)
            .round()
            .clamp(0.0, width - TICK_WIDTH);
        set_color(cr, colors.tick);
        capsule(cr, x, 0.0, TICK_WIDTH, height);
        fill(cr);
    }
}

pub fn meter(
    fraction: f64,
    tick: Option<f64>,
    colors: MeterColors,
    animate: bool,
) -> gtk::DrawingArea {
    let area = gtk::DrawingArea::new();
    area.set_content_height(METER_HEIGHT);
    area.set_hexpand(true);
    area.add_css_class("headroom-meter");
    let shown = Rc::new(Cell::new(if animate { 0.0 } else { fraction }));
    let drawn = Rc::clone(&shown);
    area.set_draw_func(move |_, cr, width, height| {
        let shape = MeterShape {
            fraction: drawn.get(),
            tick,
        };
        draw(cr, f64::from(width), f64::from(height), shape, colors);
    });
    if animate {
        motion::grow(&area, move |progress| shown.set(fraction * progress));
    }
    area
}
