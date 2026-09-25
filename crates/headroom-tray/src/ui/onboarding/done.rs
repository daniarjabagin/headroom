use std::rc::Rc;

use adw::prelude::*;

use super::{Ctx, page_column, title_block};
use crate::ui::prefs::keyboard::ShortcutRow;
use crate::ui::prefs::pill_button;

pub struct Done {
    pub widget: gtk::ScrolledWindow,
    shortcut: ShortcutRow,
}

fn session_row(ctx: &Ctx) -> adw::ActionRow {
    let row =
        adw::ActionRow::builder()
            .title(ctx.lang.tr("Start with the session"))
            .subtitle(ctx.lang.tr(
                "The tray starts after you log in and the service keeps running in the background",
            ))
            .use_markup(false)
            .build();
    row.add_prefix(&gtk::Image::from_icon_name("view-refresh-symbolic"));
    row
}

fn footnote(ctx: &Ctx) -> gtk::Label {
    let label = gtk::Label::new(Some(
        ctx.lang
            .tr("No icon? Your panel needs a system tray area (StatusNotifierItem)."),
    ));
    label.set_wrap(true);
    label.set_justify(gtk::Justification::Center);
    label.add_css_class("dim-label");
    label.add_css_class("caption");
    label
}

impl Done {
    pub fn new(ctx: &Ctx, shortcut_supported: bool) -> Self {
        let lang = ctx.lang;
        let column = page_column();
        column.append(&title_block(
            ctx,
            lang.tr("Headroom lives in your tray"),
            lang.tr(
                "The icon in the tray shows the limit that needs attention first. Click it for the details; settings are in its menu and behind the gear at the bottom of the popup.",
            ),
        ));
        let shortcut = ShortcutRow::new(lang, &ctx.act, lang.tr("Keyboard shortcut"));
        shortcut
            .row
            .add_prefix(&gtk::Image::from_icon_name("input-keyboard-symbolic"));
        shortcut.row.set_visible(shortcut_supported);
        let list = adw::PreferencesGroup::new();
        list.add(&shortcut.row);
        list.add(&session_row(ctx));
        column.append(&list);
        column.append(&footnote(ctx));
        let on_done = Rc::clone(&ctx.on_done);
        column.append(&pill_button(lang.tr("Done"), true, move || on_done()));
        let widget = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .child(&column)
            .build();
        Self { widget, shortcut }
    }

    pub fn update(&self, accelerator: &str) {
        self.shortcut.set(accelerator);
    }
}
