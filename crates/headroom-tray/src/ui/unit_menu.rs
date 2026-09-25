use std::rc::Rc;

use gtk::prelude::*;

use crate::payload::SpendUnit;
use crate::popup_model::spend_view::{unit_subtitle, unit_title, units};
use crate::ui::context::{Action, Ctx};
use crate::ui::motion::{FAST_MS, fade};
use crate::ui::widgets::{column, icon, label, row};

const CHECK_ICON: i32 = 12;
const CARET_ICON: i32 = 10;
const MENU_WIDTH: i32 = 214;

fn item(ctx: &Ctx, unit: SpendUnit, menu: &gtk::Popover) -> gtk::Button {
    let lang = ctx.locale.lang;
    let line = row(9, &[]);
    let check = icon(
        "object-select-symbolic",
        CHECK_ICON,
        &["headroom-menu-check"],
    );
    check.set_valign(gtk::Align::Start);
    check.set_opacity(if ctx.spend.is_some_and(|choice| choice.unit == unit) {
        1.0
    } else {
        0.0
    });
    line.append(&check);
    let texts = column(1, &[]);
    texts.append(&label(unit_title(lang, unit), &["headroom-menu-title"]));
    texts.append(&label(
        unit_subtitle(lang, unit),
        &["headroom-menu-subtitle"],
    ));
    line.append(&texts);
    let button = gtk::Button::new();
    button.set_child(Some(&line));
    button.add_css_class("headroom-menu-item");
    let (act, menu) = (Rc::clone(&ctx.act), menu.downgrade());
    button.connect_clicked(move |_| {
        if let Some(menu) = menu.upgrade() {
            menu.popdown();
        }
        act(Action::SelectUnit(unit));
    });
    button
}

pub fn menu(ctx: &Ctx) -> gtk::Popover {
    let popover = gtk::Popover::builder()
        .has_arrow(false)
        .css_classes(["headroom-floating", "headroom-menu"])
        .build();
    let items = column(0, &[]);
    items.set_size_request(MENU_WIDTH, -1);
    for unit in units(ctx.recent) {
        items.append(&item(ctx, unit, &popover));
    }
    popover.set_child(Some(&items));
    let motion = ctx.motion;
    popover.connect_show(move |popover| {
        if let Some(child) = popover.child() {
            fade(&child, 0.0, 1.0, FAST_MS, motion);
        }
    });
    popover
}

pub fn unit_button(ctx: &Ctx, unit: SpendUnit) -> gtk::MenuButton {
    let content = row(4, &[]);
    content.append(&label(
        unit_title(ctx.locale.lang, unit),
        &["headroom-title"],
    ));
    let caret = icon("pan-down-symbolic", CARET_ICON, &["headroom-caret-icon"]);
    caret.set_valign(gtk::Align::Center);
    content.append(&caret);
    let button = gtk::MenuButton::new();
    button.set_child(Some(&content));
    button.add_css_class("headroom-unit-button");
    button.set_popover(Some(&menu(ctx)));
    button.set_valign(gtk::Align::Center);
    button
}
