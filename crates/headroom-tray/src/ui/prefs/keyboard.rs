use std::cell::RefCell;
use std::rc::Rc;

use adw::prelude::*;
use gtk::{gdk, glib};

use super::rows::action_row;
use super::shortcut::{Capture, capture};
use super::{Act, PrefsAction};
use crate::i18n::Lang;
use crate::preferences::change::Change;

const DIALOG_WIDTH: i32 = 400;
const ILLUSTRATION_SIZE: i32 = 96;
const MARGIN: i32 = 24;

pub struct ShortcutRow {
    pub row: adw::ActionRow,
    keys: gtk::ShortcutLabel,
    current: Rc<RefCell<String>>,
}

#[must_use]
pub fn shortcut_label(lang: Lang, accelerator: &str) -> gtk::ShortcutLabel {
    let keys = gtk::ShortcutLabel::new(accelerator);
    keys.set_disabled_text(lang.tr("Disabled"));
    keys.set_valign(gtk::Align::Center);
    keys
}

impl ShortcutRow {
    pub fn new(lang: Lang, act: &Act, subtitle: &str) -> Self {
        let row = action_row(lang.tr("Open Headroom"), subtitle);
        row.set_activatable(true);
        let keys = shortcut_label(lang, "");
        row.add_suffix(&keys);
        let current: Rc<RefCell<String>> = Rc::default();
        let (act, known) = (Rc::clone(act), Rc::clone(&current));
        row.connect_activated(move |row| {
            open_capture(lang, row.upcast_ref(), &act, &known.borrow());
        });
        Self { row, keys, current }
    }

    pub fn set(&self, accelerator: &str) {
        if *self.current.borrow() != accelerator {
            accelerator.clone_into(&mut self.current.borrow_mut());
            self.keys.set_accelerator(accelerator);
        }
    }
}

fn dialog_content(lang: Lang, accelerator: &str) -> gtk::Box {
    let column = gtk::Box::new(gtk::Orientation::Vertical, 12);
    column.set_margin_start(MARGIN);
    column.set_margin_end(MARGIN);
    column.set_margin_bottom(MARGIN);
    let illustration = gtk::Image::from_icon_name("input-keyboard-symbolic");
    illustration.set_pixel_size(ILLUSTRATION_SIZE);
    illustration.add_css_class("dim-label");
    let name = gtk::Label::new(Some(lang.tr("Open Headroom")));
    name.add_css_class("dim-label");
    let prompt = gtk::Label::new(Some(lang.tr("Press your keyboard shortcut…")));
    prompt.add_css_class("title-2");
    let hint = gtk::Label::new(Some(
        lang.tr("Press Esc to cancel or Backspace to disable the keyboard shortcut."),
    ));
    hint.set_wrap(true);
    hint.set_justify(gtk::Justification::Center);
    hint.add_css_class("dim-label");
    let current = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    current.set_halign(gtk::Align::Center);
    let caption = gtk::Label::new(Some(lang.tr("Current")));
    caption.add_css_class("dim-label");
    current.append(&caption);
    current.append(&shortcut_label(lang, accelerator));
    for widget in [
        illustration.upcast_ref::<gtk::Widget>(),
        name.upcast_ref(),
        prompt.upcast_ref(),
        hint.upcast_ref(),
        current.upcast_ref(),
    ] {
        column.append(widget);
    }
    column
}

fn pressed(keyval: gdk::Key, state: gdk::ModifierType) -> Capture {
    let modifiers = state & gtk::accelerator_get_default_mod_mask();
    let key = keyval.to_lower();
    let name = key.name().map(|name| name.to_string()).unwrap_or_default();
    let accelerator = gtk::accelerator_name(key, modifiers).to_string();
    capture(&name, !modifiers.is_empty(), &accelerator)
}

pub fn open_capture(lang: Lang, parent: &gtk::Widget, act: &Act, accelerator: &str) {
    let toolbar = adw::ToolbarView::new();
    toolbar.add_top_bar(&adw::HeaderBar::new());
    toolbar.set_content(Some(&dialog_content(lang, accelerator)));
    let dialog = adw::Dialog::builder()
        .title(lang.tr("Set Shortcut"))
        .content_width(DIALOG_WIDTH)
        .child(&toolbar)
        .build();
    let keys = gtk::EventControllerKey::new();
    keys.set_propagation_phase(gtk::PropagationPhase::Capture);
    let (act, target) = (Rc::clone(act), dialog.downgrade());
    keys.connect_key_pressed(move |_, keyval, _, state| {
        let outcome = pressed(keyval, state);
        let accelerator = match outcome {
            Capture::Wait => return glib::Propagation::Stop,
            Capture::Cancel => None,
            Capture::Disable => Some(String::new()),
            Capture::Set(accelerator) => Some(accelerator),
        };
        if let Some(accelerator) = accelerator {
            act(PrefsAction::Change(Change::Shortcut(accelerator)));
        }
        if let Some(dialog) = target.upgrade() {
            dialog.close();
        }
        glib::Propagation::Stop
    });
    dialog.add_controller(keys);
    dialog.present(Some(parent));
}
