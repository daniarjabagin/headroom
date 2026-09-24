use std::f64::consts::PI;

use gtk::cairo;

use crate::palette::Rgba;

pub fn set_color(cr: &cairo::Context, color: Rgba) {
    cr.set_source_rgba(color.red, color.green, color.blue, color.alpha);
}

pub fn capsule(cr: &cairo::Context, x: f64, y: f64, width: f64, height: f64) {
    let radius = (height / 2.0).min(width / 2.0);
    cr.new_sub_path();
    cr.arc(x + width - radius, y + radius, radius, -PI / 2.0, PI / 2.0);
    cr.arc(x + radius, y + radius, radius, PI / 2.0, 3.0 * PI / 2.0);
    cr.close_path();
}

pub fn rounded_top(cr: &cairo::Context, x: f64, y: f64, width: f64, height: f64, radius: f64) {
    let radius = radius.min(width / 2.0).min(height);
    cr.new_sub_path();
    cr.move_to(x, y + height);
    cr.arc(x + radius, y + radius, radius, PI, 3.0 * PI / 2.0);
    cr.arc(x + width - radius, y + radius, radius, 3.0 * PI / 2.0, 0.0);
    cr.line_to(x + width, y + height);
    cr.close_path();
}

pub fn fill(cr: &cairo::Context) {
    if let Err(error) = cr.fill() {
        tracing::debug!(%error, "cairo could not fill a shape");
    }
}
