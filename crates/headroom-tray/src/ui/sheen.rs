use std::rc::Rc;

use gtk::gdk;
use gtk::prelude::*;

use crate::meter_shape::{Rect, sheen_rect, shows_sheen};

const RUNNING: &str = "sheen-running";

#[allow(
    clippy::cast_possible_truncation,
    reason = "meter rectangles are a few hundred pixels wide and already rounded"
)]
fn rectangle(rect: Rect) -> gdk::Rectangle {
    gdk::Rectangle::new(
        rect.x.round() as i32,
        rect.y.round() as i32,
        rect.width.round() as i32,
        rect.height.round() as i32,
    )
}

fn run_while_mapped(overlay: &gtk::Overlay) {
    overlay.connect_map(|overlay| overlay.add_css_class(RUNNING));
    overlay.connect_unmap(|overlay| overlay.remove_css_class(RUNNING));
}

#[derive(Debug, Clone, Copy)]
pub struct SheenTrack {
    pub height: f64,
    pub target: f64,
}

pub struct Sheen {
    band: gtk::Box,
}

impl Sheen {
    pub fn attach(
        overlay: &gtk::Overlay,
        track: SheenTrack,
        fraction: Rc<dyn Fn() -> f64>,
    ) -> Self {
        let band = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        band.add_css_class("headroom-sheen");
        band.set_can_target(false);
        band.set_visible(shows_sheen(track.target));
        let track = track.height;
        overlay.add_overlay(&band);
        overlay.set_clip_overlay(&band, true);
        let placed = band.clone();
        overlay.connect_get_child_position(move |overlay, child| {
            if child != placed.upcast_ref::<gtk::Widget>() {
                return None;
            }
            let (width, height) = (f64::from(overlay.width()), f64::from(overlay.height()));
            Some(
                sheen_rect(width, height, track, fraction())
                    .map_or_else(|| gdk::Rectangle::new(0, 0, 0, 0), rectangle),
            )
        });
        run_while_mapped(overlay);
        Self { band }
    }

    pub fn follow(&self, fraction: f64) {
        self.band.set_visible(shows_sheen(fraction));
        self.band.queue_resize();
    }
}
