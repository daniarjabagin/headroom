use std::cell::RefCell;
use std::rc::Rc;

use gtk::{gio, glib};

use super::App;
use crate::events::{AccountCommand, Command};
use crate::palette::Scheme;
use crate::payload::Account;
use crate::preferences::capability::Capabilities;
use crate::preferences::change::Change;
use crate::preferences::flow::{AddEvent, parse_add_event};
use crate::preferences::sync::Written;
use crate::process::ProgressProcess;
use crate::ui::prefs::diagnostics::parse_diagnostics;
use crate::ui::prefs::{PrefsAction, Service, SettingsWindow, Snapshot};
use crate::ui::style::resolve_theme;
use crate::view::View;

const LOGO_TOKEN: &str = "text-secondary";
const RESTART_COMMAND: [&str; 4] = ["systemctl", "--user", "restart", "headroom.service"];

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
        if self.capabilities().has_0_6_methods() {
            self.send(Command::GetDiagnostics);
        }
        window.present();
    }

    pub(super) fn capabilities(&self) -> Capabilities {
        Capabilities::of(self.model.borrow().view.state())
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

    pub(super) fn logo_color(&self) -> String {
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

    fn sync_prefs_later(self: &Rc<Self>) {
        let weak = Rc::downgrade(self);
        glib::idle_add_local_once(move || {
            if let Some(app) = weak.upgrade() {
                app.sync_prefs();
                app.sync_onboarding();
            }
        });
    }

    pub(super) fn apply_change(self: &Rc<Self>, change: &Change) {
        if !change.is_valid() || !self.capabilities().permits(change) {
            tracing::warn!(
                ?change,
                "ignoring a settings change the service cannot take"
            );
            self.sync_prefs_later();
            return;
        }
        let patch = self.model.borrow_mut().settings.apply(change);
        self.send(Command::UpdateSettings(patch));
        self.sync_prefs_later();
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

    pub(super) fn settings_reset(&self, result: Result<(), String>) {
        match result {
            Ok(()) => {
                self.toast(self.lang().tr("Settings reset"));
                self.send(Command::ReloadSettings);
            }
            Err(message) => self.toast(&message),
        }
    }

    pub(super) fn diagnostics_received(&self, result: Result<String, String>) {
        let parsed = result.and_then(|json| parse_diagnostics(&json).map_err(|e| e.to_string()));
        if let Some(window) = self.prefs.borrow().as_ref() {
            window.diagnostics(self.lang(), parsed);
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
        self.onboarding_restored(result);
    }

    pub(super) fn prefs_action(self: &Rc<Self>, action: PrefsAction) {
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
            PrefsAction::RestartService => self.restart_service(),
            PrefsAction::InstallUpdate => self.act(crate::ui::context::Action::InstallUpdate),
            PrefsAction::OpenUrl(url) => self.open_uri(&url),
            PrefsAction::CheckForUpdates => self.check_for_updates(),
            PrefsAction::CopyDiagnostics => self.copy_diagnostics(),
            PrefsAction::ResetSettings => self.send(Command::ResetSettings),
        }
    }

    fn copy_diagnostics(&self) {
        if let Some(window) = self.prefs.borrow().as_ref() {
            window.request_diagnostics_copy();
        }
        self.send(Command::GetDiagnostics);
    }

    fn restart_service(self: &Rc<Self>) {
        let argv: Vec<&std::ffi::OsStr> =
            RESTART_COMMAND.iter().map(std::ffi::OsStr::new).collect();
        let process = match gio::Subprocess::newv(&argv, gio::SubprocessFlags::STDERR_SILENCE) {
            Ok(process) => process,
            Err(error) => {
                self.toast(&error.to_string());
                return;
            }
        };
        self.toast(self.lang().tr("Restarting the Headroom service…"));
        let weak = Rc::downgrade(self);
        process.wait_check_async(gio::Cancellable::NONE, move |result| {
            if let (Err(error), Some(app)) = (result, weak.upgrade()) {
                tracing::warn!(%error, "the service did not restart");
                app.toast(app.lang().tr("The Headroom service could not be restarted"));
            }
        });
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
        let failure = Rc::new(RefCell::new(None::<String>));
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
