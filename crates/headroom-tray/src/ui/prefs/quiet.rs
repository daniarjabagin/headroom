use std::cell::Cell;
use std::rc::Rc;

use adw::prelude::*;

use super::rows::{ComboRow, SwitchRow};
use super::{Act, PrefsAction};
use crate::dates::Clock;
use crate::i18n::Lang;
use crate::preferences::change::Change;
use crate::preferences::model::{ClockTime, QuietHours};
use crate::preferences::options::quiet_time_choices;

type Edit = Rc<dyn Fn(&dyn Fn(&mut QuietHours))>;

pub struct QuietGroup {
    pub group: adw::PreferencesGroup,
    enabled: SwitchRow,
    from: ComboRow<ClockTime>,
    to: ComboRow<ClockTime>,
    critical: SwitchRow,
}

fn editor(act: &Act, current: &Rc<Cell<QuietHours>>) -> Edit {
    let (act, current) = (Rc::clone(act), Rc::clone(current));
    Rc::new(move |edit: &dyn Fn(&mut QuietHours)| {
        let mut quiet = current.get();
        edit(&mut quiet);
        act(PrefsAction::Change(Change::QuietHours(quiet)));
    })
}

impl QuietGroup {
    pub fn new(lang: Lang, act: &Act, current: &Rc<Cell<QuietHours>>) -> Self {
        let edit = editor(act, current);
        let (on, from, to, critical) = (
            Rc::clone(&edit),
            Rc::clone(&edit),
            Rc::clone(&edit),
            Rc::clone(&edit),
        );
        let quiet = Self {
            group: adw::PreferencesGroup::builder()
                .title(lang.tr("Quiet Hours"))
                .description(lang.tr("Held notifications arrive together when quiet hours end."))
                .build(),
            enabled: SwitchRow::new(
                lang.tr("Quiet hours"),
                lang.tr("Hold notifications while you are away"),
                move |value| on(&|quiet| quiet.enabled = value),
            ),
            from: ComboRow::new(lang.tr("From"), "", move |value| {
                from(&|quiet| quiet.from = value);
            }),
            to: ComboRow::new(lang.tr("To"), "", move |value| {
                to(&|quiet| quiet.to = value);
            }),
            critical: SwitchRow::new(
                lang.tr("Still show critical alerts"),
                lang.tr("Will run out and Almost out come through"),
                move |value| critical(&|quiet| quiet.allow_critical = value),
            ),
        };
        for row in [
            quiet.enabled.row.upcast_ref::<gtk::Widget>(),
            quiet.from.row.upcast_ref(),
            quiet.to.row.upcast_ref(),
            quiet.critical.row.upcast_ref(),
        ] {
            quiet.group.add(row);
        }
        quiet
    }

    pub fn update(&self, quiet: QuietHours, clock: Clock) {
        self.enabled.set(quiet.enabled);
        self.from
            .set(quiet_time_choices(clock, quiet.from), &quiet.from);
        self.to.set(quiet_time_choices(clock, quiet.to), &quiet.to);
        self.critical.set(quiet.allow_critical);
        for row in [
            self.from.row.upcast_ref::<gtk::Widget>(),
            self.to.row.upcast_ref(),
            self.critical.row.upcast_ref(),
        ] {
            row.set_sensitive(quiet.enabled);
        }
    }
}
