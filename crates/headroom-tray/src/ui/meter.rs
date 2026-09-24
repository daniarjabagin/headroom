use std::cell::Cell;
use std::rc::Rc;

use gtk::cairo;
use gtk::prelude::*;

use crate::meter_shape::{
    METER_HEIGHT, SEGMENT_GAP, Slot, TICK_RADIUS, TRACK_HEIGHT, fill_width, segment_layout,
    tick_rect, track_top,
};
use crate::palette::Rgba;
use crate::ui::draw::{capsule, fill, rounded_rect, set_color};
use crate::ui::motion;

#[derive(Debug, Clone, Copy)]
pub struct MeterColors {
    pub track: Rgba,
    pub fill: Rgba,
    pub tick: Rgba,
}

#[derive(Debug, Clone, Copy)]
pub struct MeterPart {
    pub fraction: f64,
    pub tick: Option<f64>,
    pub colors: MeterColors,
}

fn draw_bar(cr: &cairo::Context, slot: Slot, top: f64, part: &MeterPart) {
    set_color(cr, part.colors.track);
    capsule(cr, slot.x, top, slot.width, TRACK_HEIGHT);
    fill(cr);
    let filled = fill_width(slot.width, part.fraction);
    if filled > 0.0 {
        set_color(cr, part.colors.fill);
        capsule(cr, slot.x, top, filled, TRACK_HEIGHT);
        fill(cr);
    }
}

fn draw_tick(cr: &cairo::Context, slot: Slot, height: f64, part: &MeterPart) {
    let Some(tick) = part.tick else {
        return;
    };
    let rect = tick_rect(slot, height, tick);
    set_color(cr, part.colors.tick);
    rounded_rect(cr, rect.x, rect.y, rect.width, rect.height, TICK_RADIUS);
    fill(cr);
}

fn draw(cr: &cairo::Context, width: f64, height: f64, parts: &[MeterPart]) {
    let top = track_top(height);
    let slots = segment_layout(parts.len(), width, SEGMENT_GAP);
    for (slot, part) in slots.iter().zip(parts) {
        draw_bar(cr, *slot, top, part);
    }
    for (slot, part) in slots.iter().zip(parts) {
        draw_tick(cr, *slot, height, part);
    }
}

fn area() -> gtk::DrawingArea {
    let area = gtk::DrawingArea::new();
    area.set_content_height(METER_HEIGHT);
    area.set_hexpand(true);
    area.add_css_class("headroom-meter");
    area
}

pub fn segmented_meter(parts: Vec<MeterPart>) -> gtk::DrawingArea {
    let area = area();
    area.set_draw_func(move |_, cr, width, height| {
        draw(cr, f64::from(width), f64::from(height), &parts);
    });
    area
}

pub fn meter(
    fraction: f64,
    tick: Option<f64>,
    colors: MeterColors,
    animate: bool,
) -> gtk::DrawingArea {
    let area = area();
    let shown = Rc::new(Cell::new(if animate { 0.0 } else { fraction }));
    let drawn = Rc::clone(&shown);
    area.set_draw_func(move |_, cr, width, height| {
        let part = MeterPart {
            fraction: drawn.get(),
            tick,
            colors,
        };
        draw(cr, f64::from(width), f64::from(height), &[part]);
    });
    if animate {
        motion::grow(&area, move |progress| shown.set(fraction * progress));
    }
    area
}
