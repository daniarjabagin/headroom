use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};
use std::time::Duration;

use gtk::prelude::*;
use gtk::{cairo, glib};

use crate::palette::Rgba;
use crate::sheen_plan::{BAND_WIDTH, REST_MS, START_DELAY_MS, STEP_MS, SheenBand, sweep_progress};
use crate::ui::draw::fill;

const MICROS_PER_MS: i64 = 1000;
const HIDDEN: Rgba = Rgba {
    red: 1.0,
    green: 1.0,
    blue: 1.0,
    alpha: 0.0,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Glint {
    pub progress: f64,
    pub color: Rgba,
}

pub struct SheenClock {
    enabled: Cell<bool>,
    color: Cell<Rgba>,
    running: Cell<bool>,
    progress: Cell<Option<f64>>,
    areas: RefCell<Vec<glib::WeakRef<gtk::DrawingArea>>>,
}

impl Default for SheenClock {
    fn default() -> Self {
        Self {
            enabled: Cell::new(false),
            color: Cell::new(HIDDEN),
            running: Cell::new(false),
            progress: Cell::new(None),
            areas: RefCell::default(),
        }
    }
}

fn now_ms() -> i64 {
    glib::monotonic_time() / MICROS_PER_MS
}

fn later(clock: Weak<SheenClock>, delay_ms: u64, run: fn(&Rc<SheenClock>)) {
    glib::timeout_add_local_once(Duration::from_millis(delay_ms), move || {
        if let Some(clock) = clock.upgrade() {
            run(&clock);
        }
    });
}

impl SheenClock {
    pub fn configure(self: &Rc<Self>, enabled: bool, color: Rgba) {
        self.color.set(color);
        self.enabled.set(enabled);
        if enabled {
            self.start();
        }
    }

    pub fn attach(self: &Rc<Self>, area: &gtk::DrawingArea) {
        self.areas.borrow_mut().push(area.downgrade());
        let clock = Rc::downgrade(self);
        area.connect_map(move |_| {
            if let Some(clock) = clock.upgrade() {
                clock.start();
            }
        });
        if area.is_mapped() {
            self.start();
        }
    }

    #[must_use]
    pub fn glint(&self) -> Option<Glint> {
        let progress = self.progress.get().filter(|_| self.enabled.get())?;
        Some(Glint {
            progress,
            color: self.color.get(),
        })
    }

    fn start(self: &Rc<Self>) {
        if !self.enabled.get() || self.running.replace(true) {
            return;
        }
        later(Rc::downgrade(self), START_DELAY_MS, Self::sweep);
    }

    fn mapped(&self) -> Vec<gtk::DrawingArea> {
        let mut areas = self.areas.borrow_mut();
        areas.retain(|area| area.upgrade().is_some());
        areas
            .iter()
            .filter_map(glib::WeakRef::upgrade)
            .filter(WidgetExt::is_mapped)
            .collect()
    }

    fn can_sweep(&self) -> Option<Vec<gtk::DrawingArea>> {
        let areas = self.mapped();
        (self.enabled.get() && !areas.is_empty()).then_some(areas)
    }

    fn sweep(self: &Rc<Self>) {
        if self.can_sweep().is_none() {
            self.running.set(false);
            return;
        }
        let (started, clock) = (now_ms(), Rc::downgrade(self));
        glib::timeout_add_local(Duration::from_millis(STEP_MS), move || {
            let Some(clock) = clock.upgrade() else {
                return glib::ControlFlow::Break;
            };
            let elapsed = u64::try_from(now_ms() - started).unwrap_or(0);
            clock.step(elapsed)
        });
    }

    fn step(self: &Rc<Self>, elapsed_ms: u64) -> glib::ControlFlow {
        let areas = self.can_sweep();
        let progress = areas.as_ref().and_then(|_| sweep_progress(elapsed_ms));
        self.progress.set(progress);
        for area in areas.unwrap_or_else(|| self.mapped()) {
            area.queue_draw();
        }
        if progress.is_some() {
            return glib::ControlFlow::Continue;
        }
        if self.can_sweep().is_some() {
            later(Rc::downgrade(self), REST_MS, Self::sweep);
        } else {
            self.running.set(false);
        }
        glib::ControlFlow::Break
    }
}

pub fn paint(cr: &cairo::Context, band: SheenBand, top: f64, track: f64, color: Rgba) {
    let peak = color.alpha * band.strength;
    let gradient = cairo::LinearGradient::new(band.x, 0.0, band.x + BAND_WIDTH, 0.0);
    gradient.add_color_stop_rgba(0.0, color.red, color.green, color.blue, 0.0);
    gradient.add_color_stop_rgba(0.5, color.red, color.green, color.blue, peak);
    gradient.add_color_stop_rgba(1.0, color.red, color.green, color.blue, 0.0);
    if let Err(error) = cr.set_source(&gradient) {
        tracing::debug!(%error, "cairo could not use the sheen gradient");
        return;
    }
    cr.rectangle(band.clip_x, top, band.clip_width, track);
    fill(cr);
}
