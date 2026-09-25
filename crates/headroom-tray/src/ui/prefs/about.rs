use std::cell::RefCell;
use std::rc::Rc;

use adw::prelude::*;
use jiff::Timestamp;
use jiff::tz::TimeZone;

use super::rows::group;
use super::{Act, PrefsAction, Service, Snapshot};
use crate::dates::Locale;
use crate::i18n::Lang;
use crate::payload::Update;
use crate::update::{
    UpdateAction, UpdateRun, notes_url, update_action, update_title, whats_new_url,
};
use crate::update_check::{CheckContext, CheckRun, check_line, check_row_visible};

const LOGS_COMMAND: &str = "journalctl --user -u headroom.service -f";

struct ServiceRow {
    row: adw::ActionRow,
    spinner: gtk::Spinner,
    start: gtk::Button,
}

struct UpdateRow {
    row: adw::ActionRow,
    spinner: gtk::Spinner,
    whats_new: gtk::Button,
    action: gtk::Button,
    check: gtk::Button,
}

pub struct AboutPage {
    pub page: adw::PreferencesPage,
    service: ServiceRow,
    update: UpdateRow,
    release: Rc<RefCell<Option<Update>>>,
}

fn suffix_button(label: &str, classes: &[&str]) -> gtk::Button {
    let button = gtk::Button::with_label(label);
    button.set_valign(gtk::Align::Center);
    for class in classes {
        button.add_css_class(class);
    }
    button
}

fn action_row(title: &str, subtitle: &str) -> adw::ActionRow {
    adw::ActionRow::builder()
        .title(title)
        .subtitle(subtitle)
        .use_markup(false)
        .build()
}

fn service_row(lang: Lang, act: &Act) -> ServiceRow {
    let row = action_row(lang.tr("Headroom service"), "");
    let spinner = gtk::Spinner::new();
    let start = suffix_button(lang.tr("Start Service"), &["suggested-action"]);
    let act = Rc::clone(act);
    start.connect_clicked(move |_| act(PrefsAction::StartService));
    row.add_suffix(&spinner);
    row.add_suffix(&start);
    ServiceRow {
        row,
        spinner,
        start,
    }
}

fn logs_row(lang: Lang) -> adw::ActionRow {
    let row = action_row(lang.tr("Logs"), LOGS_COMMAND);
    row.set_subtitle_selectable(true);
    row.set_tooltip_text(Some(
        lang.tr("Run this in a terminal to follow the service"),
    ));
    let copy = suffix_button(lang.tr("Copy"), &["flat"]);
    let copied = lang.tr("Copied");
    copy.connect_clicked(move |button| {
        button.clipboard().set_text(LOGS_COMMAND);
        button.set_label(copied);
    });
    row.add_suffix(&copy);
    row
}

fn update_row(lang: Lang, act: &Act) -> UpdateRow {
    let row = action_row(lang.tr("Updates"), "");
    let spinner = gtk::Spinner::new();
    let whats_new = suffix_button(lang.tr("What's new"), &["flat"]);
    let action = suffix_button("", &[]);
    let check = suffix_button(lang.tr("Check now"), &[]);
    let act = Rc::clone(act);
    check.connect_clicked(move |_| act(PrefsAction::CheckForUpdates));
    row.add_suffix(&spinner);
    row.add_suffix(&whats_new);
    row.add_suffix(&action);
    row.add_suffix(&check);
    UpdateRow {
        row,
        spinner,
        whats_new,
        action,
        check,
    }
}

fn update_check_line(snapshot: &Snapshot) -> Option<String> {
    let state = snapshot.state?;
    let checks_on = snapshot
        .settings
        .is_some_and(|settings| settings.updates.check);
    let version = state.app_version.as_deref();
    if !check_row_visible(version, state.update_check.as_ref(), checks_on) {
        return None;
    }
    let locale = Locale::new(snapshot.lang, TimeZone::system());
    let ctx = CheckContext {
        locale: &locale,
        version: version.unwrap_or_default(),
        checked_at: state
            .update_check
            .as_ref()
            .and_then(|check| check.checked_at),
        now: Timestamp::now(),
    };
    Some(check_line(&ctx, snapshot.update_check))
}

impl AboutPage {
    pub fn new(lang: Lang, act: &Act) -> Self {
        let page = adw::PreferencesPage::builder()
            .title(lang.tr("About"))
            .icon_name("help-about-symbolic")
            .build();
        let version = action_row(lang.tr("Headroom tray"), env!("CARGO_PKG_VERSION"));
        let service = service_row(lang, act);
        let update = update_row(lang, act);
        page.add(&group(
            lang.tr("Version"),
            "",
            &[
                version.upcast_ref(),
                service.row.upcast_ref(),
                update.row.upcast_ref(),
                logs_row(lang).upcast_ref(),
            ],
        ));
        let about = Self {
            page,
            service,
            update,
            release: Rc::default(),
        };
        about.connect_update_actions(act);
        about
    }

    fn connect_update_actions(&self, act: &Act) {
        let (open, release) = (Rc::clone(act), Rc::clone(&self.release));
        self.update.whats_new.connect_clicked(move |_| {
            let url = release
                .borrow()
                .as_ref()
                .and_then(|update| whats_new_url(update).map(str::to_owned));
            if let Some(url) = url {
                open(PrefsAction::OpenUrl(url));
            }
        });
        let (act, release) = (Rc::clone(act), Rc::clone(&self.release));
        self.update.action.connect_clicked(move |button| {
            let Some(update) = release.borrow().clone() else {
                return;
            };
            match update_action(&update) {
                Some(UpdateAction::Install) => act(PrefsAction::InstallUpdate),
                Some(UpdateAction::Command) => button.clipboard().set_text(&update.command),
                Some(UpdateAction::Notes) => {
                    if let Some(url) = notes_url(&update) {
                        act(PrefsAction::OpenUrl(url.to_owned()));
                    }
                }
                None => {}
            }
        });
    }

    pub fn update(&self, snapshot: &Snapshot) {
        self.update_service(snapshot);
        let update = snapshot.state.and_then(|state| state.update.as_ref());
        let checking = snapshot
            .settings
            .is_none_or(|settings| settings.updates.check);
        self.update_release(snapshot.lang, update, checking, snapshot.update_run);
        let line = update
            .is_none()
            .then(|| update_check_line(snapshot))
            .flatten();
        self.sync_check(line.as_deref(), snapshot.update_check);
    }

    fn sync_check(&self, line: Option<&str>, run: &CheckRun) {
        let row = &self.update;
        row.check.set_visible(line.is_some());
        let Some(line) = line else {
            return;
        };
        let checking = *run == CheckRun::Checking;
        row.row.set_subtitle(line);
        row.check.set_sensitive(!checking);
        row.spinner.set_visible(checking);
        row.spinner.set_spinning(checking);
    }

    fn update_service(&self, snapshot: &Snapshot) {
        let lang = snapshot.lang;
        let running_version = snapshot
            .state
            .and_then(|state| state.app_version.as_deref());
        let (subtitle, starting, stopped) = match &snapshot.service {
            Service::Running => (
                running_version.map_or_else(
                    || lang.tr("Running").to_owned(),
                    |version| format!("{} · {version}", lang.tr("Running")),
                ),
                false,
                false,
            ),
            Service::Connecting => (lang.tr("Connecting…").to_owned(), true, false),
            Service::Stopped { starting, error } => (
                error
                    .clone()
                    .unwrap_or_else(|| lang.tr("Not running").to_owned()),
                *starting,
                !*starting,
            ),
        };
        self.service.row.set_subtitle(&subtitle);
        self.service.spinner.set_visible(starting);
        self.service.spinner.set_spinning(starting);
        self.service.start.set_visible(stopped);
    }

    fn update_release(&self, lang: Lang, update: Option<&Update>, checking: bool, run: &UpdateRun) {
        let row = &self.update;
        self.release.replace(update.cloned());
        let Some(update) = update else {
            let text = if checking {
                lang.tr("No newer release known")
            } else {
                lang.tr("Update checks are off")
            };
            row.row.set_title(lang.tr("Updates"));
            row.row.set_subtitle(text);
            for widget in [
                row.spinner.upcast_ref::<gtk::Widget>(),
                row.whats_new.upcast_ref(),
                row.action.upcast_ref(),
            ] {
                widget.set_visible(false);
            }
            return;
        };
        let action = update_action(update);
        let run = if action == Some(UpdateAction::Install) {
            run.clone()
        } else {
            UpdateRun::Idle
        };
        let idle = run == UpdateRun::Idle;
        row.row.set_title(&update_title(lang, update));
        let detail = run
            .line(lang)
            .or_else(|| (action == Some(UpdateAction::Command)).then(|| update.command.clone()));
        row.row.set_subtitle(&detail.unwrap_or_default());
        row.spinner
            .set_visible(matches!(run, UpdateRun::Running(_)));
        row.spinner
            .set_spinning(matches!(run, UpdateRun::Running(_)));
        row.whats_new
            .set_visible(idle && whats_new_url(update).is_some());
        self.sync_action(lang, action, &run);
    }

    fn sync_action(&self, lang: Lang, action: Option<UpdateAction>, run: &UpdateRun) {
        let button = &self.update.action;
        let failed = matches!(run, UpdateRun::Failed(_));
        let shown = action.is_some() && (*run == UpdateRun::Idle || failed);
        button.set_visible(shown);
        let Some(action) = action else {
            return;
        };
        button.set_label(if failed {
            lang.tr("Retry")
        } else if action == UpdateAction::Command {
            lang.tr("Copy")
        } else {
            action.label(lang)
        });
        if action == UpdateAction::Install && !failed {
            button.add_css_class("suggested-action");
        } else {
            button.remove_css_class("suggested-action");
        }
    }
}
