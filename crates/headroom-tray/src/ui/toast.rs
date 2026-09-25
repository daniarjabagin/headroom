use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use gtk::glib;
use gtk::prelude::*;

use crate::ui::motion::{SLIDE_MS, fade};
use crate::ui::widgets::{icon, label, row};

const HOLD: Duration = Duration::from_secs(2);
const OUT_MS: u32 = 150;
const ICON: i32 = 14;
const FOOTER_GAP: i32 = 14;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToastMessage {
    pub ok: bool,
    pub title: String,
    pub detail: Option<String>,
}

pub struct Toast {
    pub widget: gtk::Box,
    icon: gtk::Image,
    title: gtk::Label,
    detail: gtk::Label,
    timer: Rc<RefCell<Option<glib::SourceId>>>,
}

impl Default for Toast {
    fn default() -> Self {
        let widget = row(6, &["headroom-toast"]);
        widget.set_halign(gtk::Align::Center);
        widget.set_valign(gtk::Align::End);
        widget.set_margin_bottom(FOOTER_GAP);
        widget.set_can_target(false);
        widget.set_visible(false);
        let icon = icon("object-select-symbolic", ICON, &["headroom-toast-icon"]);
        let title = label("", &["headroom-toast-title"]);
        let detail = label("", &["headroom-toast-detail"]);
        widget.append(&icon);
        widget.append(&title);
        widget.append(&detail);
        Self {
            widget,
            icon,
            title,
            detail,
            timer: Rc::default(),
        }
    }
}

impl Toast {
    fn cancel(&self) {
        if let Some(source) = self.timer.take() {
            source.remove();
        }
    }

    pub fn show(&self, message: &ToastMessage, motion: bool) {
        self.cancel();
        let (name, class) = if message.ok {
            ("object-select-symbolic", "ok")
        } else {
            ("dialog-error-symbolic", "failed")
        };
        self.icon.set_icon_name(Some(name));
        for old in ["ok", "failed"] {
            self.icon.remove_css_class(old);
        }
        self.icon.add_css_class(class);
        self.title.set_text(&message.title);
        self.detail.set_visible(message.detail.is_some());
        self.detail.set_text(
            &message
                .detail
                .as_ref()
                .map_or(String::new(), |d| format!("· {d}")),
        );
        self.widget.set_visible(true);
        fade(&self.widget, 0.0, 1.0, SLIDE_MS, motion);
        let (widget, timer) = (self.widget.downgrade(), Rc::clone(&self.timer));
        let source = glib::timeout_add_local_once(HOLD, move || {
            timer.take();
            if let Some(widget) = widget.upgrade() {
                hide_after_fade(&widget, motion, &timer);
            }
        });
        *self.timer.borrow_mut() = Some(source);
    }

    pub fn dismiss(&self) {
        self.cancel();
        self.widget.set_visible(false);
    }
}

fn hide_after_fade(widget: &gtk::Box, motion: bool, timer: &Rc<RefCell<Option<glib::SourceId>>>) {
    if !motion {
        widget.set_visible(false);
        return;
    }
    fade(widget, 1.0, 0.0, OUT_MS, motion);
    let (hidden, pending) = (widget.downgrade(), Rc::clone(timer));
    let source =
        glib::timeout_add_local_once(Duration::from_millis(u64::from(OUT_MS)), move || {
            pending.take();
            if let Some(widget) = hidden.upgrade() {
                widget.set_visible(false);
            }
        });
    *timer.borrow_mut() = Some(source);
}
