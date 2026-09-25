use std::rc::Rc;

use gtk::glib;
use gtk::prelude::*;
use jiff::Timestamp;

use super::App;
use crate::events::{AccountCommand, Command};
use crate::i18n::fill;
use crate::palette::Scheme;
use crate::payload::{Display, SpendBreakdown, SpendPeriod, SpendUnit};
use crate::popup_model::share::{
    ShareCard, ShareInput, account_card, file_name, group_card, summary_text,
};
use crate::preferences::change::Change;
use crate::ui::context::{Action, ShareTarget};
use crate::ui::share::{folder_label, render_card, share_dir, write_atomically};
use crate::ui::style::resolve_theme;
use crate::ui::toast::ToastMessage;
use crate::view::View;

#[derive(Debug, Clone, Copy)]
enum SpendPick {
    Period(SpendPeriod),
    Unit(SpendUnit),
    Breakdown(SpendBreakdown),
}

fn starred_list(current: &[String], ids: &[String], on: bool) -> Vec<String> {
    let mut list: Vec<String> = current
        .iter()
        .filter(|id| on || !ids.contains(id))
        .cloned()
        .collect();
    if on {
        list.extend(ids.iter().filter(|id| !current.contains(id)).cloned());
    }
    list
}

impl App {
    fn speaks_0_6(&self) -> bool {
        self.model
            .borrow()
            .view
            .state()
            .is_some_and(crate::payload::State::speaks_0_6)
    }

    pub(super) fn optimistic(self: &Rc<Self>, change: &Change, edit: impl FnOnce(&mut Display)) {
        if let View::Ready(state) = &mut self.model.borrow_mut().view {
            edit(&mut state.display);
        }
        self.apply_change(change);
        self.render(false);
        self.sync_tray();
    }

    fn pick_spend(self: &Rc<Self>, pick: SpendPick) {
        if !self.speaks_0_6() {
            self.update_ui(|ui| match pick {
                SpendPick::Period(period) => ui.spend.period = Some(period),
                SpendPick::Unit(unit) => ui.spend.unit = Some(unit),
                SpendPick::Breakdown(breakdown) => ui.spend.breakdown = Some(breakdown),
            });
            return;
        }
        match pick {
            SpendPick::Period(period) => {
                self.optimistic(&Change::SpendPeriod(period), |d| d.spend_period = period);
            }
            SpendPick::Unit(unit) => {
                self.optimistic(&Change::SpendUnit(unit), |d| d.spend_unit = unit);
            }
            SpendPick::Breakdown(breakdown) => {
                let change = Change::SpendBreakdown(breakdown);
                self.optimistic(&change, |d| d.spend_breakdown = breakdown);
            }
        }
    }

    pub(super) fn popup_act(self: &Rc<Self>, action: Action) {
        match action {
            Action::SelectPeriod(period) => self.pick_spend(SpendPick::Period(period)),
            Action::SelectUnit(unit) => self.pick_spend(SpendPick::Unit(unit)),
            Action::SelectBreakdown(breakdown) => self.pick_spend(SpendPick::Breakdown(breakdown)),
            Action::SetMoreExpanded(open) => self.update_ui(|ui| ui.more_expanded = open),
            Action::HideAccounts(ids) => self.hide_accounts(ids),
            Action::SetStarred(ids, on) => self.set_starred(&ids, on),
            Action::Share(target) => self.share(&target),
            Action::CopySummary(target) => self.copy_summary(&target),
            other => tracing::debug!(?other, "not a popup action"),
        }
    }

    fn set_starred(self: &Rc<Self>, ids: &[String], on: bool) {
        let list = starred_list(&self.display().starred_accounts, ids, on);
        let change = Change::StarredAccounts(list.clone());
        self.optimistic(&change, |display| display.starred_accounts = list);
    }

    fn hide_accounts(&self, ids: Vec<String>) {
        for account_id in ids {
            self.send(Command::Account(AccountCommand::SetHidden {
                account_id,
                hidden: true,
            }));
        }
    }

    fn share_card(&self, target: &ShareTarget) -> Option<ShareCard> {
        let locale = self.locale(&self.display());
        let model = self.model.borrow();
        let state = model.view.state()?;
        let input = ShareInput {
            locale: &locale,
            display: &state.display,
            headline: state.headline.as_ref(),
            now: Timestamp::now(),
        };
        match target {
            ShareTarget::Account(id) => {
                let account = state.accounts.iter().find(|account| &account.id == id)?;
                account_card(input, account)
            }
            ShareTarget::Combined(provider) => {
                let group = state
                    .combined
                    .iter()
                    .find(|group| &group.provider == provider)?;
                group_card(input, group, &state.accounts)
            }
        }
    }

    fn popup_toast(&self, ok: bool, title: &str, detail: Option<String>) {
        let motion = self.context_motion();
        let message = ToastMessage {
            ok,
            title: title.to_owned(),
            detail,
        };
        self.tree.borrow().toast(&message, motion);
    }

    fn context_motion(&self) -> bool {
        let reduced = self
            .model
            .borrow()
            .settings
            .settings()
            .is_some_and(|settings| settings.reduced_motion);
        !crate::ui::reduced_motion(reduced)
    }

    fn copy_summary(&self, target: &ShareTarget) {
        let Some(card) = self.share_card(target) else {
            return;
        };
        let lang = self.lang();
        self.window
            .borrow()
            .clipboard()
            .set_text(&summary_text(lang, &card));
        self.popup_toast(true, lang.tr("Copied as text"), None);
    }

    fn share(&self, target: &ShareTarget) {
        let Some(card) = self.share_card(target) else {
            return;
        };
        let lang = self.lang();
        let display = self.display();
        let palette = match resolve_theme(display.theme) {
            Scheme::Light => &self.palettes.light,
            Scheme::Dark => &self.palettes.dark,
        };
        let widget = self.window.borrow().widget();
        let texture = match render_card(&widget, &card, palette, lang) {
            Ok(texture) => texture,
            Err(error) => {
                tracing::warn!(%error, "the share image could not be rendered");
                self.popup_toast(false, lang.tr("Couldn't create the image"), None);
                return;
            }
        };
        self.window.borrow().clipboard().set_texture(&texture);
        let dir = share_dir();
        let name = file_name(&card.provider, &self.locale(&display), Timestamp::now());
        let detail = match write_atomically(&dir, &name, &texture.save_to_png_bytes()) {
            Ok(path) => {
                tracing::info!(path = %path.display(), "saved a share image");
                fill(
                    lang.tr("saved to {folder}"),
                    &[("folder", &folder_label(&dir, &glib::home_dir()))],
                )
            }
            Err(error) => {
                tracing::warn!(%error, "the share image could not be saved");
                lang.tr("couldn't save the file").to_owned()
            }
        };
        self.popup_toast(true, lang.tr("Image copied"), Some(detail));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stars_are_added_once_and_removed() {
        let current = vec!["a".to_owned(), "b".to_owned()];
        let ids = vec!["b".to_owned(), "c".to_owned()];
        assert_eq!(starred_list(&current, &ids, true), ["a", "b", "c"]);
        assert_eq!(starred_list(&current, &ids, false), ["a"]);
    }
}
