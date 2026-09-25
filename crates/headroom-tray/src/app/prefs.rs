use std::rc::Rc;

use super::App;
use crate::events::{AccountCommand, Command};
use crate::palette::Scheme;
use crate::payload::Account;
use crate::preferences::change::Change;
use crate::preferences::flow::{AddEvent, parse_add_event};
use crate::preferences::sync::Written;
use crate::process::ProgressProcess;
use crate::ui::prefs::{PrefsAction, Service, SettingsWindow, Snapshot};
use crate::ui::style::resolve_theme;
use crate::view::View;

const LOGO_TOKEN: &str = "text-secondary";

impl App {
    pub(super) fn open_settings(self: &Rc<Self>) {
        let existing = self.prefs.borrow().clone();
        let window = existing.unwrap_or_else(|| {
            let weak = Rc::downgrade(self);
            let window = SettingsWindow::new(
                Some(&self.application),
                Rc::new(move |action| {
                    if let Some(app) = weak.upgrade() {
                        app.prefs_action(action);
                    }
                }),
            );
            *self.prefs.borrow_mut() = Some(Rc::clone(&window));
            window
        });
        self.window.borrow().hide();
        self.sync_prefs();
        window.present();
    }

    fn service(&self) -> Service {
        let model = self.model.borrow();
        match &model.view {
            View::Loading => Service::Connecting,
            View::Unavailable { .. } => Service::Stopped {
                starting: model.ui.service_starting,
                error: model.ui.service_error.clone(),
            },
            View::Failed(_) | View::Ready(_) => Service::Running,
        }
    }

    fn logo_color(&self) -> String {
        let display = self.display();
        let palette = match resolve_theme(display.theme) {
            Scheme::Light => &self.palettes.light,
            Scheme::Dark => &self.palettes.dark,
        };
        palette
            .css_value(LOGO_TOKEN)
            .map_or_else(|_| String::from("#808080"), str::to_owned)
    }

    pub(super) fn sync_prefs(&self) {
        let Some(window) = self.prefs.borrow().clone() else {
            return;
        };
        let (lang, logo_color, service) = (self.lang(), self.logo_color(), self.service());
        let model = self.model.borrow();
        let snapshot = Snapshot {
            lang,
            logo_color,
            service,
            settings: model.settings.settings(),
            state: model.view.state(),
            providers: model.providers.as_ref(),
            update_run: &model.ui.update_run,
            update_check: &model.ui.update_check,
        };
        window.update(&snapshot);
    }

    pub(super) fn apply_change(&self, change: &Change) {
        let patch = self.model.borrow_mut().settings.apply(change);
        self.send(Command::UpdateSettings(patch));
        self.sync_prefs();
    }

    pub(super) fn settings_written(self: &Rc<Self>, result: Result<(), String>) {
        let written = self.model.borrow_mut().settings.written();
        if let Err(message) = result {
            tracing::warn!(%message, "a settings change failed");
            self.toast(&message);
            self.send(Command::ReloadSettings);
        } else if written == Written::Settled {
            self.send(Command::ReloadSettings);
        }
    }

    pub(super) fn toast(&self, message: &str) {
        if let Some(window) = self.prefs.borrow().as_ref() {
            window.toast(message);
        }
    }

    pub(super) fn restored(&self, result: &Result<(), String>) {
        if let Some(window) = self.prefs.borrow().as_ref() {
            window.restored(result);
        }
    }

    fn prefs_action(self: &Rc<Self>, action: PrefsAction) {
        match action {
            PrefsAction::Change(change) => self.apply_change(&change),
            PrefsAction::SetLabel { account_id, label } => {
                self.send(Command::Account(AccountCommand::SetLabel {
                    account_id,
                    label,
                }));
            }
            PrefsAction::SetHidden { account_id, hidden } => {
                self.send(Command::Account(AccountCommand::SetHidden {
                    account_id,
                    hidden,
                }));
            }
            PrefsAction::SetOrder(ids) => {
                self.send(Command::Account(AccountCommand::SetOrder(ids)));
            }
            PrefsAction::Restore(provider) => {
                self.send(Command::Account(AccountCommand::Restore(provider)));
            }
            PrefsAction::Remove(account) => self.remove_account(&account),
            PrefsAction::StartService => self.act(crate::ui::context::Action::StartService),
            PrefsAction::InstallUpdate => self.act(crate::ui::context::Action::InstallUpdate),
            PrefsAction::OpenUrl(url) => self.open_uri(&url),
            PrefsAction::CheckForUpdates => self.check_for_updates(),
        }
    }

    fn open_uri(&self, url: &str) {
        let window = self
            .prefs
            .borrow()
            .as_ref()
            .map(|prefs| prefs.window().clone());
        gtk::UriLauncher::new(url).launch(window.as_ref(), gtk::gio::Cancellable::NONE, |result| {
            if let Err(error) = result {
                tracing::warn!(%error, "could not open a link");
            }
        });
    }

    fn remove_account(self: &Rc<Self>, account: &Account) {
        let failure = Rc::new(std::cell::RefCell::new(None::<String>));
        let (seen, weak) = (Rc::clone(&failure), Rc::downgrade(self));
        let started = ProgressProcess::start(
            &[
                "accounts",
                "remove",
                &account.id,
                "--yes",
                "--progress",
                "json",
            ],
            move |line| {
                if let Some(AddEvent::Error { message }) = parse_add_event(line) {
                    *seen.borrow_mut() = Some(message);
                }
            },
            move |result| {
                let Some(app) = weak.upgrade() else {
                    return;
                };
                let lang = app.lang();
                let message = failure
                    .borrow_mut()
                    .take()
                    .or_else(|| result.err().map(|error| error.message(lang)))
                    .unwrap_or_else(|| lang.tr("Account removed").to_owned());
                app.toast(&message);
            },
        );
        if let Err(error) = started {
            self.toast(&error.message(self.lang()));
        }
    }
}
