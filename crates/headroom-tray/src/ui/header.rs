use gtk::prelude::*;

use crate::assets::{CLAUDE_COLOR, Tint, provider_logo};
use crate::popup_model::links::{QuickLink, quick_links};
use crate::ui::context::{Action, Ctx};
use crate::ui::header_menu::{MenuContext, MenuInput, open_menu};
use crate::ui::link_icons::link_icon;
use crate::ui::widgets::{button, label, row, spacer};

const PROVIDER_ICON: i32 = 16;
const COMPACT_PROVIDER_ICON: i32 = 14;
const LINK_ICON: i32 = 14;
const HOVERED: &str = "hovered";

pub struct HeaderInput {
    pub title: String,
    pub plan: Option<String>,
    pub status: Vec<gtk::Widget>,
    pub menu: MenuInput,
}

pub fn provider_icon_sized(ctx: &Ctx, provider: &str, size: i32) -> gtk::Image {
    let (svg, tint) = provider_logo(provider);
    let color = match tint {
        Tint::Brand => CLAUDE_COLOR.to_owned(),
        Tint::Text => ctx.css("text-secondary"),
    };
    ctx.svg_image(svg, &color, size, &["headroom-provider-icon"])
}

pub fn provider_icon(ctx: &Ctx, provider: &str) -> gtk::Image {
    let size = if ctx.compact() {
        COMPACT_PROVIDER_ICON
    } else {
        PROVIDER_ICON
    };
    provider_icon_sized(ctx, provider, size)
}

pub fn links_of(ctx: &Ctx, provider: &str) -> Vec<QuickLink> {
    ctx.links.get(provider).map(quick_links).unwrap_or_default()
}

fn link_buttons(ctx: &Ctx, links: &[QuickLink]) -> gtk::Box {
    let icons = row(2, &["headroom-link-icons"]);
    let color = ctx.css("text-secondary");
    for link in links {
        let glyph = link_icon(&ctx.textures, &color, link.kind, LINK_ICON);
        let open = button(
            &glyph,
            &["headroom-icon-button", "headroom-link-button"],
            ctx.action(Action::OpenUrl(link.url.clone())),
        );
        open.set_tooltip_text(Some(&link.tooltip(ctx.locale.lang)));
        open.set_valign(gtk::Align::Center);
        icons.append(&open);
    }
    icons
}

fn reveal_on_hover(line: &gtk::Box) {
    let motion = gtk::EventControllerMotion::new();
    let (entered, left) = (line.downgrade(), line.downgrade());
    motion.connect_enter(move |_, _, _| {
        if let Some(line) = entered.upgrade() {
            line.add_css_class(HOVERED);
        }
    });
    motion.connect_leave(move |_| {
        if let Some(line) = left.upgrade() {
            line.remove_css_class(HOVERED);
        }
    });
    line.add_controller(motion);
}

fn context_menu(ctx: &Ctx, line: &gtk::Box, input: MenuInput) {
    let click = gtk::GestureClick::new();
    click.set_button(gtk::gdk::BUTTON_SECONDARY);
    let (menu, anchor) = (MenuContext::of(ctx), line.downgrade());
    click.connect_pressed(move |gesture, _, x, y| {
        gesture.set_state(gtk::EventSequenceState::Claimed);
        if let Some(anchor) = anchor.upgrade() {
            open_menu(&menu, &anchor, &input, (x, y));
        }
    });
    line.add_controller(click);
}

pub fn section_header(ctx: &Ctx, provider: &str, input: HeaderInput) -> gtk::Box {
    let line = row(6, &["headroom-section-header"]);
    line.append(&provider_icon(ctx, provider));
    let title = label(&input.title, &["headroom-title"]);
    title.set_ellipsize(gtk::pango::EllipsizeMode::End);
    line.append(&title);
    if let Some(plan) = &input.plan {
        let plan = label(plan, &["headroom-plan"]);
        plan.set_valign(gtk::Align::Baseline);
        plan.set_ellipsize(gtk::pango::EllipsizeMode::End);
        line.append(&plan);
    }
    for status in &input.status {
        status.set_valign(gtk::Align::Center);
        line.append(status);
    }
    line.append(&spacer());
    if !input.menu.links.is_empty() {
        line.append(&link_buttons(ctx, &input.menu.links));
        reveal_on_hover(&line);
    }
    context_menu(ctx, &line, input.menu);
    line
}
