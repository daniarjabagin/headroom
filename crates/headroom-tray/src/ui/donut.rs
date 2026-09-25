use std::cell::Cell;
use std::rc::Rc;

use gtk::cairo;
use gtk::prelude::*;

use crate::donut::{Arc, geometry, sector_path, segments, visible_fractions};
use crate::palette::Rgba;
use crate::popup_model::spend_view::RingCenter;
use crate::ui::draw::{fill, set_color};
use crate::ui::motion;
use crate::ui::widgets::{column, label};

const DONUT_SIZE: i32 = 104;
const COMPACT_DONUT_SIZE: i32 = 84;

fn trace(cr: &cairo::Context, arcs: &[Arc]) {
    for arc in arcs {
        if arc.new_sub_path {
            cr.new_sub_path();
        }
        let (x, y) = arc.center;
        if arc.negative {
            cr.arc_negative(x, y, arc.radius, arc.from, arc.to);
        } else {
            cr.arc(x, y, arc.radius, arc.from, arc.to);
        }
    }
    cr.close_path();
}

fn draw(cr: &cairo::Context, size: f64, fractions: &[f64], colors: &[Rgba], reveal: f64) {
    let geometry = geometry(size);
    cr.set_antialias(cairo::Antialias::Best);
    for segment in segments(&sweep(fractions, reveal)) {
        let Some(arcs) = sector_path(&geometry, &segment) else {
            continue;
        };
        if let Some(color) = colors.get(segment.index) {
            set_color(cr, *color);
            trace(cr, &arcs);
            fill(cr);
        }
    }
}

fn sweep(fractions: &[f64], reveal: f64) -> Vec<f64> {
    let mut left = reveal;
    fractions
        .iter()
        .map(|fraction| {
            let shown = fraction.min(left.max(0.0));
            left -= fraction;
            shown
        })
        .collect()
}

fn center_label(center: &RingCenter) -> gtk::Box {
    let texts = column(0, &["headroom-donut-center"]);
    texts.set_halign(gtk::Align::Center);
    texts.set_valign(gtk::Align::Center);
    let amount = label(&center.amount, &["headroom-donut-value"]);
    amount.set_xalign(0.5);
    texts.append(&amount);
    if let Some(unit) = center.unit_line {
        let unit = label(unit, &["headroom-donut-unit"]);
        unit.set_xalign(0.5);
        texts.append(&unit);
    }
    texts
}

pub fn donut(
    values: &[f64],
    colors: Vec<Rgba>,
    center: &RingCenter,
    animate: bool,
    compact: bool,
) -> gtk::Overlay {
    let fractions = visible_fractions(values);
    let size = if compact {
        COMPACT_DONUT_SIZE
    } else {
        DONUT_SIZE
    };
    let area = gtk::DrawingArea::new();
    area.set_content_width(size);
    area.set_content_height(size);
    let reveal = Rc::new(Cell::new(if animate { 0.0 } else { 1.0 }));
    let shown = Rc::clone(&reveal);
    area.set_draw_func(move |_, cr, width, height| {
        let size = f64::from(width.min(height));
        draw(cr, size, &fractions, &colors, shown.get());
    });
    if animate {
        motion::grow(&area, move |progress| reveal.set(progress));
    }
    let overlay = gtk::Overlay::new();
    overlay.set_child(Some(&area));
    overlay.set_valign(gtk::Align::Center);
    overlay.add_overlay(&center_label(center));
    overlay
}
