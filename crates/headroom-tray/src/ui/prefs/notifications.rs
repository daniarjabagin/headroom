use std::cell::Cell;
use std::rc::Rc;

use adw::prelude::*;

use super::picks::almost_out_subtitle;
use super::quiet::QuietGroup;
use super::rows::{SwitchRow, changer, group};
use super::thresholds::ThresholdGroup;
use super::{Act, Snapshot};
use crate::dates::Clock;
use crate::i18n::{Lang, system_clock};
use crate::preferences::capability::Capabilities;
use crate::preferences::change::{Change, Milestone};
use crate::preferences::model::{Notifications, QuietHours};
use crate::preferences::notify::DEFAULT_THRESHOLD;

pub struct NotificationsPage {
    pub page: adw::PreferencesPage,
    rows: Vec<(Milestone, SwitchRow)>,
    thresholds: ThresholdGroup,
    quiet: QuietGroup,
    quiet_now: Rc<Cell<QuietHours>>,
    lang: Lang,
}

fn texts(lang: Lang, milestone: Milestone) -> (&'static str, &'static str) {
    match milestone {
        Milestone::AlmostOut => (lang.tr("Almost out"), ""),
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

fn milestone_rows(lang: Lang, act: &Act) -> Vec<(Milestone, SwitchRow)> {
    Milestone::ALL
        .into_iter()
        .map(|milestone| {
            let (title, subtitle) = texts(lang, milestone);
            let row = SwitchRow::new(
                title,
                subtitle,
                changer(act, move |on| Change::Notify(milestone, on)),
            );
            (milestone, row)
        })
        .collect()
}

impl NotificationsPage {
    pub fn new(lang: Lang, act: &Act, logo_color: &str) -> Self {
        let page = adw::PreferencesPage::builder()
            .title(lang.tr("Notifications"))
            .icon_name("preferences-system-notifications-symbolic")
            .build();
        let rows = milestone_rows(lang, act);
        let widgets: Vec<&gtk::Widget> = rows.iter().map(|(_, row)| row.row.upcast_ref()).collect();
        page.add(&group(
            lang.tr("Notify Me When"),
            lang.tr("Hidden accounts and hidden limits never notify."),
            &widgets,
        ));
        let quiet_now = Rc::new(Cell::new(QuietHours::default()));
        let thresholds = ThresholdGroup::new(lang, act, logo_color);
        let quiet = QuietGroup::new(lang, act, &quiet_now);
        page.add(&thresholds.group);
        page.add(&quiet.group);
        Self {
            page,
            rows,
            thresholds,
            quiet,
            quiet_now,
            lang,
        }
    }

    pub fn update(&self, snapshot: &Snapshot) {
        let (Some(settings), Some(state)) = (snapshot.settings, snapshot.state) else {
            return;
        };
        let notifications = &settings.notifications;
        let capable = Capabilities::of(Some(state)).release_0_6;
        for (milestone, row) in &self.rows {
            row.set(enabled(notifications, *milestone));
        }
        let threshold = if capable {
            notifications.threshold_percent
        } else {
            DEFAULT_THRESHOLD
        };
        if let Some((_, almost)) = self.rows.first() {
            almost
                .row
                .set_subtitle(&almost_out_subtitle(self.lang, threshold));
        }
        self.thresholds.group.set_visible(capable);
        self.quiet.group.set_visible(capable);
        if !capable {
            return;
        }
        self.thresholds.update(notifications, state);
        self.quiet_now.set(notifications.quiet_hours);
        let clock = Clock::resolve(settings.display.time_format, system_clock());
        self.quiet.update(notifications.quiet_hours, clock);
    }
}
