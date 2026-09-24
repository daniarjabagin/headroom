use std::rc::Rc;

use adw::prelude::*;

use super::rows::pill_button;
use super::{Act, PrefsAction, Service};
use crate::assets::MARK;
use crate::i18n::Lang;
use crate::ui::widgets::svg_texture;

const MARK_SIZE: i32 = 96;

pub struct ServicePage {
    pub page: adw::PreferencesPage,
    status: adw::StatusPage,
    start: gtk::Button,
    spinner: gtk::Spinner,
    error: gtk::Label,
}

impl ServicePage {
    pub fn new(lang: Lang, act: &Act, logo_color: &str) -> Self {
        let page = adw::PreferencesPage::builder()
            .title(lang.tr("Service"))
            .icon_name("system-run-symbolic")
            .build();
        let status = adw::StatusPage::builder().vexpand(true).build();
        if let Some(mark) = svg_texture(MARK, logo_color, MARK_SIZE) {
            status.set_paintable(Some(&mark));
        }
        let act = Rc::clone(act);
        let start = pill_button(lang.tr("Start Service"), true, move || {
            act(PrefsAction::StartService);
        });
        let spinner = gtk::Spinner::new();
        let error = gtk::Label::new(None);
        error.set_wrap(true);
        error.add_css_class("error");
        let column = gtk::Box::new(gtk::Orientation::Vertical, 12);
        column.set_halign(gtk::Align::Center);
        column.append(&start);
        column.append(&spinner);
        column.append(&error);
        status.set_child(Some(&column));
        let group = adw::PreferencesGroup::new();
        group.add(&status);
        page.add(&group);
        Self {
            page,
            status,
            start,
            spinner,
            error,
        }
    }

    pub fn update(&self, lang: Lang, service: &Service) {
        let (title, description, busy, error) = match service {
            Service::Connecting | Service::Running => {
                (lang.tr("Connecting to Headroom…"), "", true, None)
            }
            Service::Stopped { starting, error } => (
                lang.tr("Headroom service isn't running"),
                lang.tr("Settings live in the Headroom service. Start it to change them."),
                *starting,
                error.as_deref(),
            ),
        };
        self.status.set_title(title);
        self.status.set_description(Some(description));
        self.start.set_visible(!busy);
        self.spinner.set_visible(busy);
        self.spinner.set_spinning(busy);
        self.error.set_visible(error.is_some());
        self.error.set_text(error.unwrap_or_default());
    }
}
