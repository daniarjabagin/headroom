use gtk::gdk;
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};

use crate::placement::{self, LayerPlacement, POPUP_WIDTH, Rect};

const NAMESPACE: &str = "headroom";

#[must_use]
pub fn supported() -> bool {
    gtk4_layer_shell::is_supported()
}

pub fn init(window: &gtk::Window) {
    window.init_layer_shell();
    window.set_layer(Layer::Top);
    window.set_namespace(Some(NAMESPACE));
    window.set_exclusive_zone(0);
    window.set_keyboard_mode(KeyboardMode::OnDemand);
}

fn apply(window: &gtk::Window, placement: LayerPlacement) {
    let (anchored, free) = match placement.edge {
        placement::Edge::Top => (Edge::Top, Edge::Bottom),
        placement::Edge::Bottom => (Edge::Bottom, Edge::Top),
    };
    window.set_anchor(anchored, true);
    window.set_anchor(free, false);
    window.set_anchor(Edge::Left, true);
    window.set_anchor(Edge::Right, false);
    window.set_margin(anchored, placement.margin_edge);
    window.set_margin(Edge::Left, placement.margin_left);
}

pub fn place(window: &gtk::Window, monitor: &gdk::Monitor, area: Rect, click: Option<(i32, i32)>) {
    window.set_monitor(Some(monitor));
    apply(window, placement::layer_placement(area, click, POPUP_WIDTH));
}
