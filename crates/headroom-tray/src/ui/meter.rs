use std::cell::{Cell, RefCell};
use std::rc::Rc;

use adw::prelude::*;
use gtk::cairo;

use crate::meter_shape::{
    METER_HEIGHT, SEGMENT_GAP, Slot, TICK_RADIUS, TRACK_HEIGHT, fill_width, segment_layout,
    tick_rect,
};
use crate::palette::Rgba;
use crate::ui::draw::{capsule, fill, rounded_rect, set_color};
use crate::ui::motion;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MeterColors {
    pub track: Rgba,
    pub fill: Rgba,
    pub tick: Rgba,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MeterSize {
    pub track: f64,
    pub height: i32,
}

const NORMAL: MeterSize = MeterSize {
    track: TRACK_HEIGHT,
    height: METER_HEIGHT,
};
const COMPACT: MeterSize = MeterSize {
    track: 4.0,
    height: 8,
};

#[must_use]
pub fn meter_size(compact: bool) -> MeterSize {
    if compact { COMPACT } else { NORMAL }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MeterPart {
    pub fraction: f64,
    pub tick: Option<f64>,
    pub colors: MeterColors,
}

fn draw_bar(cr: &cairo::Context, slot: Slot, top: f64, track: f64, part: &MeterPart) {
    set_color(cr, part.colors.track);
    capsule(cr, slot.x, top, slot.width, track);
    fill(cr);
    let filled = fill_width(slot.width, part.fraction);
    if filled > 0.0 {
        set_color(cr, part.colors.fill);
        capsule(cr, slot.x, top, filled, track);
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

fn draw(cr: &cairo::Context, width: f64, height: f64, track: f64, parts: &[MeterPart]) {
    let top = ((height - track) / 2.0).round();
    let slots = segment_layout(parts.len(), width, SEGMENT_GAP);
    for (slot, part) in slots.iter().zip(parts) {
        draw_bar(cr, *slot, top, track, part);
    }
    for (slot, part) in slots.iter().zip(parts) {
        draw_tick(cr, *slot, height, part);
    }
}

fn area(size: MeterSize) -> gtk::DrawingArea {
    let area = gtk::DrawingArea::new();
    area.set_content_height(size.height);
    area.set_hexpand(true);
    area.add_css_class("headroom-meter");
    area
}

pub struct SegmentedMeter {
    pub area: gtk::DrawingArea,
    parts: Rc<RefCell<Vec<MeterPart>>>,
}

impl SegmentedMeter {
    pub fn new(parts: Vec<MeterPart>, size: MeterSize) -> Self {
        let area = area(size);
        let parts = Rc::new(RefCell::new(parts));
        let drawn = Rc::clone(&parts);
        area.set_draw_func(move |_, cr, width, height| {
            draw(
                cr,
                f64::from(width),
                f64::from(height),
                size.track,
                &drawn.borrow(),
            );
        });
        Self { area, parts }
    }

    pub fn set(&self, parts: Vec<MeterPart>) {
        if *self.parts.borrow() == parts {
            return;
        }
        *self.parts.borrow_mut() = parts;
        self.area.queue_draw();
    }
}

struct Shown {
    fraction: Cell<f64>,
    from: Cell<f64>,
    part: Cell<MeterPart>,
    grown: Cell<bool>,
}

impl Shown {
    fn step(&self, progress: f64) {
        let (from, to) = (self.from.get(), self.part.get().fraction);
        self.fraction.set(from + (to - from) * progress);
    }
}

pub struct Meter {
    pub area: gtk::DrawingArea,
    shown: Rc<Shown>,
    animation: adw::TimedAnimation,
}

impl Meter {
    pub fn new(part: MeterPart, animate: bool, size: MeterSize) -> Self {
        let area = area(size);
        let shown = Rc::new(Shown {
            fraction: Cell::new(if animate { 0.0 } else { part.fraction }),
            from: Cell::new(0.0),
            part: Cell::new(part),
            grown: Cell::new(!animate),
        });
        let drawn = Rc::clone(&shown);
        area.set_draw_func(move |_, cr, width, height| {
            let part = MeterPart {
                fraction: drawn.fraction.get(),
                ..drawn.part.get()
            };
            draw(cr, f64::from(width), f64::from(height), size.track, &[part]);
        });
        let stepped = Rc::clone(&shown);
        let animation = motion::progress_animation(&area, move |progress| stepped.step(progress));
        if animate {
            let (first, grown) = (animation.clone(), Rc::clone(&shown));
            motion::on_first_map(&area, move || {
                grown.grown.set(true);
                first.play();
            });
        }
        Self {
            area,
            shown,
            animation,
        }
    }

    pub fn set(&self, part: MeterPart, motion: bool) {
        let old = self.shown.part.get();
        if old == part {
            return;
        }
        self.shown.part.set(part);
        if !self.shown.grown.get() {
            return;
        }
        let moved = (old.fraction - part.fraction).abs() > f64::EPSILON;
        if motion && moved && self.area.is_mapped() {
            self.shown.from.set(self.shown.fraction.get());
            self.animation.play();
        } else {
            self.animation.reset();
            self.shown.fraction.set(part.fraction);
        }
        self.area.queue_draw();
    }
}
