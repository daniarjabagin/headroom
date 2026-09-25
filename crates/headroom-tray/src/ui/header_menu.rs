use std::rc::Rc;

use gtk::glib::WeakRef;
use gtk::prelude::*;

use crate::i18n::{Lang, fill};
use crate::popup_model::links::{LinkKind, QuickLink};
use crate::ui::context::{Action, Ctx, ShareTarget};
use crate::ui::link_icons::link_icon;
use crate::ui::motion::{FAST_MS, fade};
use crate::ui::popover::{point_at, transient};
use crate::ui::widgets::{Textures, column, icon, label, row};

const ITEM_ICON: i32 = 14;
const CHECK_ICON: i32 = 12;

#[derive(Clone)]
pub struct MenuContext {
    pub lang: Lang,
    pub recent: bool,
    pub motion: bool,
    pub icon_color: String,
    pub act: Rc<dyn Fn(Action)>,
    pub textures: Rc<Textures>,
}

impl MenuContext {
    pub fn of(ctx: &Ctx) -> Self {
        Self {
            lang: ctx.locale.lang,
            recent: ctx.recent,
            motion: ctx.motion,
            icon_color: ctx.css("text-secondary"),
            act: Rc::clone(&ctx.act),
            textures: Rc::clone(&ctx.textures),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuInput {
    pub provider_name: String,
    pub account_ids: Vec<String>,
    pub starred: bool,
    pub target: ShareTarget,
    pub links: Vec<QuickLink>,
}

struct Entry {
    icon: gtk::Widget,
    title: String,
    suffix: Suffix,
    actions: Vec<Action>,
}

enum Suffix {
    Nothing,
    Detail(String),
    Check,
}

fn named(name: &str) -> gtk::Widget {
    icon(name, ITEM_ICON, &["headroom-menu-icon"]).upcast()
}

fn check_mark() -> gtk::Image {
    let check = icon(
        "object-select-symbolic",
        CHECK_ICON,
        &["headroom-menu-check"],
    );
    check.set_valign(gtk::Align::Center);
    check.set_margin_start(12);
    check
}

fn account_entries(ctx: &MenuContext, input: &MenuInput) -> Vec<Entry> {
    let lang = ctx.lang;
    let ids = &input.account_ids;
    let mut entries = vec![
        Entry {
            icon: named("view-refresh-symbolic"),
            title: fill(
                lang.tr("Refresh {provider}"),
                &[("provider", &input.provider_name)],
            ),
            suffix: Suffix::Nothing,
            actions: ids.iter().map(|id| Action::Refresh(id.clone())).collect(),
        },
        Entry {
            icon: named("view-conceal-symbolic"),
            title: lang.tr("Hide from popup").to_owned(),
            suffix: Suffix::Nothing,
            actions: vec![Action::HideAccounts(ids.clone())],
        },
    ];
    if ctx.recent {
        entries.push(Entry {
            icon: named("starred-symbolic"),
            title: lang.tr("Always show").to_owned(),
            suffix: if input.starred {
                Suffix::Check
            } else {
                Suffix::Nothing
            },
            actions: vec![Action::SetStarred(ids.clone(), !input.starred)],
        });
    }
    entries
}

fn link_entries(ctx: &MenuContext, links: &[QuickLink]) -> Vec<Entry> {
    let order = [LinkKind::Status, LinkKind::Dashboard, LinkKind::Usage];
    order
        .iter()
        .filter_map(|kind| links.iter().find(|link| link.kind == *kind))
        .map(|link| Entry {
            icon: link_icon(&ctx.textures, &ctx.icon_color, link.kind, ITEM_ICON).upcast(),
            title: link.kind.title(ctx.lang).to_owned(),
            suffix: Suffix::Detail(link.host.clone()),
            actions: vec![Action::OpenUrl(link.url.clone())],
        })
        .collect()
}

fn share_entries(lang: Lang, target: &ShareTarget) -> Vec<Entry> {
    vec![
        Entry {
            icon: named("send-to-symbolic"),
            title: lang.tr("Share as image…").to_owned(),
            suffix: Suffix::Nothing,
            actions: vec![Action::Share(target.clone())],
        },
        Entry {
            icon: named("edit-copy-symbolic"),
            title: lang.tr("Copy as text").to_owned(),
            suffix: Suffix::Nothing,
            actions: vec![Action::CopySummary(target.clone())],
        },
    ]
}

fn item(ctx: &MenuContext, entry: Entry, menu: &WeakRef<gtk::Popover>) -> gtk::Button {
    let line = row(9, &[]);
    entry.icon.set_valign(gtk::Align::Center);
    line.append(&entry.icon);
    let title = label(&entry.title, &["headroom-menu-title"]);
    title.set_hexpand(true);
    line.append(&title);
    match &entry.suffix {
        Suffix::Nothing => {}
        Suffix::Detail(detail) => line.append(&label(detail, &["headroom-menu-detail"])),
        Suffix::Check => line.append(&check_mark()),
    }
    let button = gtk::Button::new();
    button.set_child(Some(&line));
    button.add_css_class("headroom-menu-item");
    let (act, menu, actions) = (Rc::clone(&ctx.act), menu.clone(), entry.actions);
    button.connect_clicked(move |_| {
        if let Some(menu) = menu.upgrade() {
            menu.popdown();
        }
        for action in &actions {
            act(action.clone());
        }
    });
    button
}

fn separator() -> gtk::Separator {
    let line = gtk::Separator::new(gtk::Orientation::Horizontal);
    line.add_css_class("headroom-menu-separator");
    line
}

fn fill_menu(ctx: &MenuContext, input: &MenuInput, items: &gtk::Box, menu: &WeakRef<gtk::Popover>) {
    let groups = [
        account_entries(ctx, input),
        link_entries(ctx, &input.links),
        share_entries(ctx.lang, &input.target),
    ];
    let mut first = true;
    for group in groups.into_iter().filter(|group| !group.is_empty()) {
        if !first {
            items.append(&separator());
        }
        first = false;
        for entry in group {
            items.append(&item(ctx, entry, menu));
        }
    }
}

pub fn menu_items(ctx: &MenuContext, input: &MenuInput, menu: &gtk::Popover) -> gtk::Box {
    let items = column(0, &[]);
    fill_menu(ctx, input, &items, &menu.downgrade());
    items
}

pub fn open_menu(ctx: &MenuContext, anchor: &gtk::Box, input: &MenuInput, at: (f64, f64)) {
    let placeholder = column(0, &[]);
    let popover = transient(anchor, &placeholder);
    popover.add_css_class("headroom-menu");
    let items = menu_items(ctx, input, &popover);
    popover.set_child(Some(&items));
    point_at(&popover, at.0, at.1);
    popover.set_position(gtk::PositionType::Bottom);
    popover.popup();
    fade(&items, 0.0, 1.0, FAST_MS, ctx.motion);
}
