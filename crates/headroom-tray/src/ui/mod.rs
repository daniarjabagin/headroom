mod account_section;
mod breakdown;
mod collapsed;
mod combined_section;
pub mod context;
mod donut;
mod draw;
mod footer;
mod header;
mod header_menu;
mod keyed;
#[cfg(feature = "layer-shell")]
mod layer;
mod link_icons;
mod meter;
mod model_popover;
mod motion;
mod notice;
mod popover;
pub mod popup;
pub mod popup_tree;
pub mod prefs;
pub mod preview;
mod quota_row;
mod section_card;
pub mod share;
mod spend_card;
mod status_notice;
mod status_views;
pub mod style;
pub mod toast;
mod unit_menu;
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
pub fn render(
    tree: &mut PopupTree,
    ctx: Ctx,
    view: &View,
    frame: &Frame,
    rebuild: bool,
) -> (Option<gtk::Box>, Vec<Tick>) {
    let root = tree.render(&ctx, view, frame, rebuild);
    let mut ticks = tree.ticks();
    ticks.extend(ctx.ticks.into_inner());
    (root, ticks)
}
