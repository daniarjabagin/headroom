use std::cell::RefCell;
use std::rc::Rc;

use adw::prelude::*;
use gtk::{gio, glib};

use super::diagnostics::expand_home;
use super::rows::{ComboRow, action_row, changer, icon_button, suffix_button};
use super::{Act, PrefsAction, Service, Snapshot};
use crate::i18n::Lang;
use crate::preferences::change::Change;
use crate::preferences::model::LogLevel;
use crate::preferences::options::log_level_choices;

const RESET: &str = "reset";
const CANCEL: &str = "cancel";

pub type Toast = Rc<dyn Fn(&str)>;

pub struct AdvancedPage {
    pub page: adw::PreferencesPage,
    service: adw::ActionRow,
    restart: gtk::Button,
    level: ComboRow<LogLevel>,
    log_file: adw::ActionRow,
    log_buttons: gtk::Box,
    log_path: Rc<RefCell<Option<String>>>,
    lang: Lang,
}

fn service_group(lang: Lang, act: &Act) -> (adw::PreferencesGroup, adw::ActionRow, gtk::Button) {
    let row = action_row(lang.tr("Headroom service"), "");
    let restart = suffix_button(lang.tr("Restart"), &[]);
    let act = Rc::clone(act);
    restart.connect_clicked(move |_| act(PrefsAction::RestartService));
    row.add_suffix(&restart);
    let group = adw::PreferencesGroup::builder()
        .title(lang.tr("Service"))
        .build();
    group.add(&row);
    (group, row, restart)
}

fn open_folder(path: &str, parent: Option<&gtk::Window>) {
    let file = gio::File::for_path(expand_home(path, &glib::home_dir()));
    gtk::FileLauncher::new(Some(&file)).open_containing_folder(
        parent,
        gio::Cancellable::NONE,
        |result| {
            if let Err(error) = result {
                tracing::warn!(%error, "could not open the log folder");
            }
        },
    );
}

fn log_buttons(lang: Lang, path: &Rc<RefCell<Option<String>>>, toast: &Toast) -> gtk::Box {
    let buttons = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    buttons.set_valign(gtk::Align::Center);
    let copy = icon_button("edit-copy-symbolic", lang.tr("Copy path"));
    let (known, toast) = (Rc::clone(path), Rc::clone(toast));
    copy.connect_clicked(move |button| {
        if let Some(path) = known.borrow().as_deref() {
            button.clipboard().set_text(path);
            toast(lang.tr("Path copied"));
        }
    });
    let open = icon_button("folder-open-symbolic", lang.tr("Open folder"));
    let known = Rc::clone(path);
    open.connect_clicked(move |button| {
        if let Some(path) = known.borrow().as_deref() {
            open_folder(path, button.root().and_downcast::<gtk::Window>().as_ref());
        }
    });
    buttons.append(&copy);
    buttons.append(&open);
    buttons
}

fn troubleshooting_group(lang: Lang, act: &Act) -> adw::PreferencesGroup {
    let row = action_row(
        lang.tr("Copy diagnostics"),
        lang.tr("Versions, desktop and account states. No tokens or emails."),
    );
    let copy = suffix_button(lang.tr("Copy"), &[]);
    let act = Rc::clone(act);
    copy.connect_clicked(move |_| act(PrefsAction::CopyDiagnostics));
    row.add_suffix(&copy);
    let group = adw::PreferencesGroup::builder()
        .title(lang.tr("Troubleshooting"))
        .description(lang.tr("Paste the diagnostics into a bug report."))
        .build();
    group.add(&row);
    group
}

fn confirm_reset(lang: Lang, parent: &gtk::Widget, act: &Act) {
    let dialog = adw::AlertDialog::new(
        Some(lang.tr("Reset all settings?")),
        Some(lang.tr(
            "Accounts stay signed in; appearance, notifications and hidden limits return to defaults.",
        )),
    );
    dialog.add_responses(&[(CANCEL, lang.tr("Cancel")), (RESET, lang.tr("Reset"))]);
    dialog.set_response_appearance(RESET, adw::ResponseAppearance::Destructive);
    dialog.set_default_response(Some(CANCEL));
    dialog.set_close_response(CANCEL);
    let act = Rc::clone(act);
    dialog.connect_response(None, move |_, response| {
        if response == RESET {
            act(PrefsAction::ResetSettings);
        }
    });
    dialog.present(Some(parent));
}

fn reset_group(lang: Lang, act: &Act) -> adw::PreferencesGroup {
    let label = gtk::Label::new(Some(lang.tr("Reset all settings…")));
    label.add_css_class("error");
    label.add_css_class("heading");
    label.set_margin_top(14);
    label.set_margin_bottom(14);
    let row = gtk::ListBoxRow::builder()
        .child(&label)
        .activatable(true)
        .build();
    let act = Rc::clone(act);
    row.connect_activate(move |row| confirm_reset(lang, row.upcast_ref(), &act));
    let group = adw::PreferencesGroup::new();
    group.add(&row);
    group
}

impl AdvancedPage {
    pub fn new(lang: Lang, act: &Act, toast: &Toast) -> Self {
        let page = adw::PreferencesPage::builder()
            .title(lang.tr("Advanced"))
            .icon_name("emblem-system-symbolic")
            .build();
        let (service_group, service, restart) = service_group(lang, act);
        let level = ComboRow::new(
            lang.tr("Log level"),
            lang.tr("Debug adds provider responses without tokens"),
            changer(act, Change::LogLevel),
        );
        let log_path: Rc<RefCell<Option<String>>> = Rc::default();
        let log_file = action_row(lang.tr("Log file"), "");
        let log_buttons = log_buttons(lang, &log_path, toast);
        log_file.add_suffix(&log_buttons);
        let logging = adw::PreferencesGroup::builder()
            .title(lang.tr("Logging"))
            .build();
        logging.add(&level.row);
        logging.add(&log_file);
        page.add(&service_group);
        page.add(&logging);
        page.add(&troubleshooting_group(lang, act));
        page.add(&reset_group(lang, act));
        Self {
            page,
            service,
            restart,
            level,
            log_file,
            log_buttons,
            log_path,
            lang,
        }
    }

    pub fn update(&self, snapshot: &Snapshot) {
        let lang = self.lang;
        let version = snapshot
            .state
            .and_then(|state| state.app_version.as_deref());
        let running = snapshot.service == Service::Running;
        let status = if running {
            lang.tr("Running")
        } else {
            lang.tr("Not running")
        };
        let mut parts = vec![status.to_owned()];
        parts.extend(version.map(|version| format!("{} {version}", lang.tr("version"))));
        parts.push(lang.tr("systemd user service").to_owned());
        self.service.set_subtitle(&parts.join(" · "));
        self.restart.set_sensitive(running);
        if let Some(settings) = snapshot.settings {
            self.level
                .set(log_level_choices(lang), &settings.logging.level);
        }
    }

    pub fn set_log_file(&self, path: Option<&str>) {
        self.log_file
            .set_subtitle(path.unwrap_or_else(|| self.lang.tr("Not available")));
        self.log_buttons.set_visible(path.is_some());
        *self.log_path.borrow_mut() = path.map(str::to_owned);
    }
}
