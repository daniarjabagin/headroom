mod account_section;
mod combined_section;
pub mod context;
mod donut;
mod draw;
mod footer;
#[cfg(feature = "layer-shell")]
mod layer;
mod meter;
mod motion;
mod notice;
pub mod popup;
pub mod popup_tree;
pub mod prefs;
mod quota_row;
mod section_card;
mod spend_card;
mod status_views;
pub mod style;
mod update_row;
mod usage_rows;
mod widgets;
pub mod window;
mod x11;

use crate::view::View;
use context::{Ctx, Tick};
use popup::Frame;
use popup_tree::PopupTree;

pub use motion::reduced as reduced_motion;
pub use widgets::svg_texture_at;

#[must_use]
pub fn build(ctx: Ctx, view: &View, frame: &Frame) -> (gtk::Box, Vec<Tick>) {
    let root = PopupTree::default().mount(&ctx, view, frame);
    (root, ctx.ticks.into_inner())
}

#[must_use]
pub fn render(
    tree: &mut PopupTree,
    ctx: Ctx,
    view: &View,
    frame: &Frame,
    rebuild: bool,
) -> (Option<gtk::Box>, Vec<Tick>) {
    let root = tree.render(&ctx, view, frame, rebuild);
    (root, ctx.ticks.into_inner())
}
