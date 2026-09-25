use std::rc::Rc;

use adw::prelude::*;

use super::about::update_check_line;
use super::keyboard::ShortcutRow;
use super::rows::{SwitchRow, action_row, changer, suffix_button};
use super::shortcut;
use super::{Act, PrefsAction, Snapshot};
use crate::i18n::Lang;
use crate::preferences::capability::Capabilities;
use crate::preferences::change::Change;
use crate::update_check::CheckRun;

struct CheckRow {
    row: adw::ActionRow,
    spinner: gtk::Spinner,
    button: gtk::Button,
}

pub struct PrivacyGroup {
    pub group: adw::PreferencesGroup,
    pub keyboard: adw::PreferencesGroup,
    check_updates: SwitchRow,
    check: CheckRow,
    status_pages: SwitchRow,
    shortcut: ShortcutRow,
}

fn check_row(lang: Lang, act: &Act) -> CheckRow {
    let row = action_row(lang.tr("Latest version"), "");
    let spinner = gtk::Spinner::new();
    let button = suffix_button(lang.tr("Check now"), &[]);
    let act = Rc::clone(act);
    button.connect_clicked(move |_| act(PrefsAction::CheckForUpdates));
    row.add_suffix(&spinner);
    row.add_suffix(&button);
    CheckRow {
        row,
        spinner,
        button,
    }
}

impl PrivacyGroup {
    pub fn new(lang: Lang, act: &Act) -> Self {
        let privacy = Self {
            group: adw::PreferencesGroup::builder()
                .title(lang.tr("Privacy"))
                .build(),
            keyboard: adw::PreferencesGroup::builder()
                .title(lang.tr("Keyboard"))
                .build(),
            check_updates: SwitchRow::new(
                lang.tr("Check for updates"),
                lang.tr("Once a day, asks GitHub for the latest release. Nothing else is sent."),
                changer(act, Change::CheckUpdates),
            ),
            check: check_row(lang, act),
            status_pages: SwitchRow::new(
                lang.tr("Status pages"),
                lang.tr("Show provider incidents from public status pages"),
                changer(act, Change::StatusPages),
            ),
            shortcut: ShortcutRow::new(lang, act, lang.tr("Opens the popup from any app")),
        };
        privacy.group.add(&privacy.check_updates.row);
        privacy.group.add(&privacy.check.row);
        privacy.group.add(&privacy.status_pages.row);
        privacy.keyboard.add(&privacy.shortcut.row);
        privacy
    }

    pub fn update(&self, snapshot: &Snapshot) {
        let Some(settings) = snapshot.settings else {
            return;
        };
        let capable = Capabilities::of(snapshot.state).release_0_6;
        self.check_updates.set(settings.updates.check);
        self.status_pages.row.set_visible(capable);
        self.status_pages.set(settings.status_pages.enabled);
        self.keyboard.set_visible(capable && shortcut::support());
        self.shortcut.set(&settings.shortcuts.open);
        let line = update_check_line(snapshot);
        self.check.row.set_visible(line.is_some());
        if let Some(line) = line {
            let checking = *snapshot.update_check == CheckRun::Checking;
            self.check.row.set_subtitle(&line);
            self.check.button.set_sensitive(!checking);
            self.check.spinner.set_visible(checking);
            self.check.spinner.set_spinning(checking);
        }
    }
}
