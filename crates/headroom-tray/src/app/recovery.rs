use std::rc::Rc;

use gtk::glib;

use super::App;
use crate::events::Command;
use crate::ui::prefs::target::provider_of_account;
use crate::update_check::{CheckOutcome, CheckRun, parse_outcome};

const COPIED_SECONDS: u32 = 2;

impl App {
    pub(super) fn retry(self: &Rc<Self>, account_id: String) {
        self.update_ui(|ui| {
            ui.retrying.insert(account_id.clone());
        });
        self.send(Command::Refresh(account_id));
    }

    pub(super) fn retry_settled(self: &Rc<Self>, account_id: &str) {
        let removed = self.model.borrow_mut().ui.retrying.remove(account_id);
        if removed {
            self.render(false);
        }
    }

    pub(super) fn sign_in_again(self: &Rc<Self>, account_id: &str) {
        self.open_settings();
        let Some(window) = self.prefs.borrow().clone() else {
            return;
        };
        window.focus_account(account_id);
        if self.capabilities().has_0_6_methods() {
            let provider = self.provider_of(account_id);
            if !window.open_login_dialog(&provider, account_id) {
                tracing::debug!(account_id, "no sign-in flow to repeat for this account");
            }
        }
    }

    fn provider_of(&self, account_id: &str) -> String {
        self.model
            .borrow()
            .view
            .state()
            .and_then(|state| {
                state
                    .accounts
                    .iter()
                    .find(|account| account.id == account_id)
            })
            .map_or_else(
                || provider_of_account(account_id).to_owned(),
                |account| account.provider.clone(),
            )
    }

    pub(super) fn copy_command(self: &Rc<Self>, account_id: String, command: &str) {
        self.window.borrow().clipboard().set_text(command);
        let copied = account_id.clone();
        self.update_ui(|ui| ui.copied_command = Some(copied));
        let weak = Rc::downgrade(self);
        glib::timeout_add_seconds_local_once(COPIED_SECONDS, move || {
            let Some(app) = weak.upgrade() else {
                return;
            };
            let current = app.model.borrow().ui.copied_command == Some(account_id);
            if current {
                app.update_ui(|ui| ui.copied_command = None);
            }
        });
    }

    pub(super) fn check_for_updates(self: &Rc<Self>) {
        if self.model.borrow().ui.update_check == CheckRun::Checking {
            return;
        }
        self.update_ui(|ui| ui.update_check = CheckRun::Checking);
        self.send(Command::CheckForUpdates);
    }

    fn last_checked(&self) -> Option<jiff::Timestamp> {
        self.model
            .borrow()
            .view
            .state()
            .and_then(|state| state.update_check.as_ref())
            .and_then(|check| check.checked_at)
    }

    pub(super) fn update_checked(self: &Rc<Self>, result: Result<String, String>) {
        let outcome = result
            .and_then(|json| parse_outcome(&json).map_err(|error| error.to_string()))
            .unwrap_or_else(|message| {
                tracing::warn!(%message, "the update check failed");
                CheckOutcome::failed(self.last_checked())
            });
        self.update_ui(|ui| ui.update_check = CheckRun::Done(outcome));
    }
}
