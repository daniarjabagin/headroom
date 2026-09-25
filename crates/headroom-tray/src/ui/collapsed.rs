use gtk::prelude::*;

use crate::popup_model::collapse::MoreSummary;
use crate::ui::context::{Action, Ctx};
use crate::ui::header::provider_icon_sized;
use crate::ui::widgets::{button, icon, label, row};

const GLYPH: i32 = 14;
const CHEVRON: i32 = 12;
const SHOW_LESS_ICON: i32 = 10;

pub fn more_row(ctx: &Ctx, summary: &MoreSummary) -> gtk::Button {
    let line = row(8, &[]);
    let glyphs = row(4, &[]);
    for provider in &summary.providers {
        glyphs.append(&provider_icon_sized(ctx, provider, GLYPH));
    }
    line.append(&glyphs);
    line.append(&label(&summary.title, &["headroom-more-title"]));
    let names = label(&format!("· {}", summary.names), &["headroom-more-names"]);
    names.set_hexpand(true);
    names.set_ellipsize(gtk::pango::EllipsizeMode::End);
    line.append(&names);
    line.append(&icon("pan-end-symbolic", CHEVRON, &["headroom-caret-icon"]));
    let more = button(
        &line,
        &["headroom-more-row"],
        ctx.action(Action::SetMoreExpanded(true)),
    );
    more.set_tooltip_text(Some(&summary.names));
    more
}

pub fn less_divider(ctx: &Ctx) -> gtk::Box {
    let lang = ctx.locale.lang;
    let line = row(8, &["headroom-more-divider"]);
    line.append(&label(
        lang.tr("Not pinned"),
        &["headroom-more-divider-title"],
    ));
    let rule = gtk::Separator::new(gtk::Orientation::Horizontal);
    rule.set_hexpand(true);
    rule.set_valign(gtk::Align::Center);
    rule.add_css_class("headroom-breakdown-separator");
    line.append(&rule);
    let content = row(4, &[]);
    content.append(&label(lang.tr("Show less"), &["headroom-more-less"]));
    let caret = icon("pan-up-symbolic", SHOW_LESS_ICON, &["headroom-caret-icon"]);
    caret.set_valign(gtk::Align::Center);
    content.append(&caret);
    let less = button(
        &content,
        &["headroom-more-less-button"],
        ctx.action(Action::SetMoreExpanded(false)),
    );
    line.append(&less);
    line
}
