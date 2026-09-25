use std::rc::Rc;

use adw::prelude::*;

use super::rows::{SwitchRow, group};
use super::{Act, PrefsAction, Snapshot};
use crate::i18n::Lang;
use crate::preferences::change::{Change, Milestone};
use crate::preferences::model::Notifications;

pub struct NotificationsPage {
    pub page: adw::PreferencesPage,
    rows: Vec<(Milestone, SwitchRow)>,
}

fn texts(lang: Lang, milestone: Milestone) -> (&'static str, &'static str) {
    match milestone {
        Milestone::AlmostOut => (
            lang.tr("Almost out"),
            lang.tr("A limit drops under 10% left"),
        ),
        Milestone::CuttingItClose => (
            lang.tr("Cutting it close"),
            lang.tr("The pace says a limit will barely last until reset"),
        ),
        Milestone::WillRunOut => (
            lang.tr("Will run out"),
            lang.tr("The pace says a limit runs out before it resets"),
        ),
        Milestone::Reset => (
            lang.tr("Limit reset"),
            lang.tr("A limit that was running low resets"),
        ),
    }
}

fn enabled(notifications: &Notifications, milestone: Milestone) -> bool {
    match milestone {
        Milestone::AlmostOut => notifications.almost_out,
        Milestone::CuttingItClose => notifications.cutting_it_close,
        Milestone::WillRunOut => notifications.will_run_out,
        Milestone::Reset => notifications.reset,
    }
}

impl NotificationsPage {
    pub fn new(lang: Lang, act: &Act) -> Self {
        let page = adw::PreferencesPage::builder()
            .title(lang.tr("Notifications"))
            .icon_name("preferences-system-notifications-symbolic")
            .build();
        let rows: Vec<(Milestone, SwitchRow)> = Milestone::ALL
            .into_iter()
            .map(|milestone| {
                let (title, subtitle) = texts(lang, milestone);
                let act = Rc::clone(act);
                let row = SwitchRow::new(title, subtitle, move |on| {
                    act(PrefsAction::Change(Change::Notify(milestone, on)));
                });
                (milestone, row)
            })
            .collect();
        let widgets: Vec<&gtk::Widget> = rows.iter().map(|(_, row)| row.row.upcast_ref()).collect();
        page.add(&group(
            lang.tr("Notify Me When"),
            lang.tr("Hidden accounts and hidden limits never notify."),
            &widgets,
        ));
        Self { page, rows }
    }

    pub fn update(&self, snapshot: &Snapshot) {
        let Some(settings) = snapshot.settings else {
            return;
        };
        for (milestone, row) in &self.rows {
            row.set(enabled(&settings.notifications, *milestone));
        }
    }
}
