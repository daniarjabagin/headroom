use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use gtk::prelude::*;
use gtk::{gdk, glib};

use crate::palette::Rgba;
use crate::ui::draw::{capsule, fill, set_color};

const DWELL: Duration = Duration::from_millis(400);
const BAR_HEIGHT: i32 = 3;
const BAR_GAP: f64 = 1.0;

fn release(popover: &gtk::Popover) {
    if popover.parent().is_some() {
        popover.unparent();
    }
}

pub fn transient(parent: &impl IsA<gtk::Widget>, child: &impl IsA<gtk::Widget>) -> gtk::Popover {
    let popover = gtk::Popover::builder()
        .has_arrow(false)
        .css_classes(["headroom-floating"])
        .build();
    popover.set_child(Some(child));
    popover.set_parent(parent);
    let weak = popover.downgrade();
    let unrealized = parent.connect_unrealize(move |_| {
        if let Some(popover) = weak.upgrade() {
            release(&popover);
        }
    });
    let owner = parent.clone().upcast::<gtk::Widget>();
    let handler = RefCell::new(Some(unrealized));
    popover.connect_closed(move |popover| {
        if let Some(id) = handler.take() {
            owner.disconnect(id);
        }
        let popover = popover.clone();
        glib::idle_add_local_once(move || release(&popover));
    });
    popover
}

pub fn point_at(popover: &gtk::Popover, x: f64, y: f64) {
    #[allow(
        clippy::cast_possible_truncation,
        reason = "pointer positions are small pixel values"
    )]
    let rect = gdk::Rectangle::new(x as i32, y as i32, 1, 1);
    popover.set_pointing_to(Some(&rect));
}

pub fn beside_window(popover: &gtk::Popover, anchor: &gtk::Widget) {
    let Some(root) = anchor.root() else {
        return;
    };
    let origin = anchor
        .compute_point(&root, &gtk::graphene::Point::new(0.0, 0.0))
        .map_or(0.0, |point| f64::from(point.x()));
    #[allow(
        clippy::cast_possible_truncation,
        reason = "window offsets are small pixel values"
    )]
    let left = -(origin as i32);
    popover.set_pointing_to(Some(&gdk::Rectangle::new(left, 0, 1, anchor.height())));
    popover.set_position(gtk::PositionType::Left);
}

pub fn on_dwell(widget: &impl IsA<gtk::Widget>, open: impl Fn() -> Option<gtk::Popover> + 'static) {
    let timer: Rc<RefCell<Option<glib::SourceId>>> = Rc::default();
    let shown: Rc<RefCell<Option<gtk::Popover>>> = Rc::default();
    let motion = gtk::EventControllerMotion::new();
    let open = Rc::new(open);
    let (enter_timer, enter_shown) = (Rc::clone(&timer), Rc::clone(&shown));
    motion.connect_enter(move |_, _, _| {
        let (pending, shown, open) = (
            Rc::clone(&enter_timer),
            Rc::clone(&enter_shown),
            Rc::clone(&open),
        );
        let source = glib::timeout_add_local_once(DWELL, move || {
            pending.take();
            *shown.borrow_mut() = open();
        });
        if let Some(old) = enter_timer.replace(Some(source)) {
            old.remove();
        }
    });
    let close = move || {
        if let Some(source) = timer.take() {
            source.remove();
        }
        if let Some(popover) = shown.take() {
            popover.popdown();
        }
    };
    let close = Rc::new(close);
    let leave = Rc::clone(&close);
    motion.connect_leave(move |_| leave());
    widget.connect_unmap(move |_| close());
    widget.add_controller(motion);
}

pub fn share_bar(parts: Vec<(Rgba, f64)>, track: Rgba) -> gtk::DrawingArea {
    let area = gtk::DrawingArea::new();
    area.set_content_height(BAR_HEIGHT);
    area.set_hexpand(true);
    area.set_valign(gtk::Align::Center);
    area.set_draw_func(move |_, cr, width, height| {
        let (width, height) = (f64::from(width), f64::from(height));
        set_color(cr, track);
        capsule(cr, 0.0, 0.0, width, height);
        fill(cr);
        if cr.save().is_err() {
            return;
        }
        capsule(cr, 0.0, 0.0, width, height);
        cr.clip();
        let mut x = 0.0;
        for (color, fraction) in &parts {
            let span = (width * fraction).max(0.0);
            if span <= 0.0 {
                continue;
            }
            set_color(cr, *color);
            cr.rectangle(x, 0.0, (span - BAR_GAP).max(1.0), height);
            fill(cr);
            x += span;
        }
        if let Err(error) = cr.restore() {
            tracing::debug!(%error, "cairo could not restore a share bar");
        }
    });
    area
}
