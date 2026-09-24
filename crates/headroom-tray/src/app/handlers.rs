use std::rc::Rc;

use gtk::glib;
use gtk::prelude::*;

use super::App;
use super::subprocess::{launch, run_update};
use crate::events::{Command, Event, MenuAction};
use crate::i18n::{Lang, system_locale};
use crate::payload::parse_state;
use crate::providers::{sign_in_command, terminal_sign_in};
use crate::settings::{
    reduced_motion_setting, toggled_reset_format_patch, toggled_value_mode_patch,
};
use crate::ui::context::Action;
use crate::update::UpdateRun;
use crate::view::View;

const FAILED_REFRESH_SECONDS: u32 = 3;

impl App {
    pub(super) fn handle(self: &Rc<Self>, event: Event) {
        match event {
            Event::State(json) => self.receive_state(&json),
            Event::Settings(json) => self.receive_settings(&json),
            Event::Providers(json) => self.model.borrow_mut().sign_in = terminal_sign_in(&json),
            Event::Vanished => self.set_view(View::Unavailable {
                starting: false,
                error: None,
            }),
            Event::CallFailed(message) => self.call_failed(message),
            Event::RefreshSettled(succeeded) => self.refresh_settled(succeeded),
            Event::ServiceStarted(result) => self.service_started(result),
            Event::OpenRequested | Event::Menu(MenuAction::Open) => self.show(None),
            Event::Activate { x, y } => self.toggle(Some((x, y))),
            Event::Menu(MenuAction::RefreshNow) => self.act(Action::RefreshNow),
            Event::Menu(MenuAction::Quit) => self.application.quit(),
            Event::TrayFailed(message) => {
                tracing::warn!(%message, "the tray icon is unavailable; use `headroom-tray --toggle`");
            }
        }
    }

    fn set_view(self: &Rc<Self>, view: View) {
        self.model.borrow_mut().view = view;
        self.render(false);
        self.sync_tray();
    }

    fn receive_state(self: &Rc<Self>, json: &str) {
        let view = match parse_state(json) {
            Ok(state) => {
                tracing::debug!(accounts = state.accounts.len(), "state received");
                View::Ready(Box::new(state))
            }
            Err(error) => View::Failed(error.to_string()),
        };
        self.model.borrow_mut().ui.service_starting = false;
        self.set_view(view);
    }

    fn receive_settings(self: &Rc<Self>, json: &str) {
        let reduced = reduced_motion_setting(json);
        let changed = {
            let mut model = self.model.borrow_mut();
            let changed = model.reduced_motion_setting != reduced;
            model.reduced_motion_setting = reduced;
            changed
        };
        if changed {
            self.render(false);
        }
    }

    fn call_failed(self: &Rc<Self>, message: String) {
        tracing::warn!(%message, "a call to the Headroom service failed");
        if matches!(self.model.borrow().view, View::Loading) {
            self.set_view(View::Failed(message));
        }
    }

    fn refresh_settled(self: &Rc<Self>, succeeded: bool) {
        {
            let mut model = self.model.borrow_mut();
            model.refresh_pressed = false;
            model.refresh_failed = !succeeded;
        }
        self.render(false);
        if succeeded {
            return;
        }
        let weak = Rc::downgrade(self);
        glib::timeout_add_seconds_local_once(FAILED_REFRESH_SECONDS, move || {
            if let Some(app) = weak.upgrade() {
                app.model.borrow_mut().refresh_failed = false;
                app.render(false);
            }
        });
    }

    fn service_started(self: &Rc<Self>, result: Result<(), String>) {
        {
            let mut model = self.model.borrow_mut();
            model.ui.service_starting = false;
            model.ui.service_error = result.err();
        }
        self.render(false);
    }

    fn lang(&self) -> Lang {
        let language = self
            .model
            .borrow()
            .view
            .state()
            .map_or(crate::payload::Language::System, |state| {
                state.display.language
            });
        Lang::resolve(language, system_locale().as_deref())
    }

    fn patch(&self, patch: impl Fn(&crate::payload::Display) -> String) {
        let patch = self
            .model
            .borrow()
            .view
            .state()
            .map(|state| patch(&state.display));
        if let Some(patch) = patch {
            self.send(Command::UpdateSettings(patch));
        }
    }

    pub(super) fn act(self: &Rc<Self>, action: Action) {
        match action {
            Action::RefreshNow => self.refresh_now(),
            Action::Refresh(id) => self.send(Command::Refresh(id)),
            Action::ToggleValueMode => self.patch(toggled_value_mode_patch),
            Action::ToggleResetFormat => self.patch(toggled_reset_format_patch),
            Action::SelectPeriod(period) => self.update_ui(|ui| ui.period = period),
            Action::SetExpanded(id, open) => {
                let expanded = &mut self.model.borrow_mut().ui.expanded;
                if open {
                    expanded.insert(id)
                } else {
                    expanded.remove(&id)
                };
            }
            Action::StartService => self.start_service(),
            Action::InstallUpdate => self.install_update(),
            Action::ToggleUpdateCommand => self.update_ui(|ui| {
                ui.update_command_open = !ui.update_command_open;
                ui.copied = false;
            }),
            Action::Copy(text) => {
                self.window.borrow().clipboard().set_text(&text);
                self.update_ui(|ui| ui.copied = true);
            }
            Action::OpenUrl(url) => self.window.borrow().open_uri(&url),
            Action::SignIn(provider) => self.sign_in(&provider),
            Action::Quit => self.application.quit(),
        }
    }

    fn update_ui(self: &Rc<Self>, change: impl FnOnce(&mut crate::ui::context::UiState)) {
        change(&mut self.model.borrow_mut().ui);
        self.render(false);
    }

    fn refresh_now(self: &Rc<Self>) {
        self.model.borrow_mut().refresh_pressed = true;
        self.send(Command::RefreshNow);
        self.render(false);
    }

    fn start_service(self: &Rc<Self>) {
        self.update_ui(|ui| {
            ui.service_starting = true;
            ui.service_error = None;
        });
        self.send(Command::StartService);
    }

    fn sign_in(&self, provider: &str) {
        if let Some(command) = sign_in_command(provider, self.lang().tr("Press Enter to close")) {
            launch(&command);
            self.window.borrow().hide();
        }
    }

    fn install_update(self: &Rc<Self>) {
        if matches!(self.model.borrow().ui.update_run, UpdateRun::Running(_)) {
            return;
        }
        self.update_ui(|ui| ui.update_run = UpdateRun::Running(None));
        let (on_event, on_exit) = (Rc::downgrade(self), Rc::downgrade(self));
        run_update(
            move |event| {
                if let Some(app) = on_event.upgrade() {
                    app.update_ui(|ui| {
                        ui.update_run = std::mem::take(&mut ui.update_run).after_event(event);
                    });
                }
            },
            move |error| {
                if let Some(app) = on_exit.upgrade() {
                    let lang = app.lang();
                    app.update_ui(|ui| {
                        ui.update_run = std::mem::take(&mut ui.update_run).after_exit(lang, error);
                    });
                }
            },
        );
    }
}
