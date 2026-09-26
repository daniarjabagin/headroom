use adw::prelude::*;

use super::Act;
use super::rows::{ComboRow, SegmentedRow, SwitchRow, changer};
use crate::i18n::Lang;
use crate::payload::{SpendBreakdown, SpendPeriod, SpendUnit, State};
use crate::preferences::change::{Change, Section};
use crate::preferences::model::Settings;
use crate::preferences::options::{
    spend_breakdown_choices, spend_period_choices, spend_unit_choices,
};

pub struct SpendGroup {
    pub group: adw::PreferencesGroup,
    show: SwitchRow,
    period: ComboRow<SpendPeriod>,
    unit: SegmentedRow<SpendUnit>,
    show_breakdown: SwitchRow,
    breakdown: SegmentedRow<SpendBreakdown>,
    lang: Lang,
}

impl SpendGroup {
    pub fn new(lang: Lang, act: &Act) -> Self {
        let spend = Self {
            group: adw::PreferencesGroup::builder()
                .title(lang.tr("Spend"))
                .build(),
            show: SwitchRow::new(
                lang.tr("Show spend"),
                lang.tr("Spend ring for all tools at the top of the popup"),
                changer(act, |on| Change::Section(Section::Spend, on)),
            ),
            period: ComboRow::new(
                lang.tr("Default period"),
                lang.tr("The tab the spend ring opens on"),
                changer(act, Change::SpendPeriod),
            ),
            unit: SegmentedRow::new(
                lang.tr("Units"),
                "",
                spend_unit_choices(lang),
                changer(act, Change::SpendUnit),
            ),
            show_breakdown: SwitchRow::new(
                lang.tr("Show models and projects"),
                lang.tr("Model and project lists in the spend card"),
                changer(act, Change::ShowBreakdown),
            ),
            breakdown: SegmentedRow::new(
                lang.tr("Breakdown on hover"),
                lang.tr("Split a slice of the ring when the pointer rests on it"),
                spend_breakdown_choices(lang, None),
                changer(act, Change::SpendBreakdown),
            ),
            lang,
        };
        for row in [
            spend.show.row.upcast_ref::<gtk::Widget>(),
            spend.period.row.upcast_ref(),
            spend.unit.row.upcast_ref(),
            spend.show_breakdown.row.upcast_ref(),
            spend.breakdown.row.upcast_ref(),
        ] {
            spend.group.add(row);
        }
        spend
    }

    pub fn update(&self, settings: &Settings, state: &State) {
        let display = &settings.display;
        self.show.set(display.show_spend);
        self.period.set(
            spend_period_choices(self.lang, Some(&state.spend)),
            &display.spend_period,
        );
        self.unit.set(&display.spend_unit);
        self.show_breakdown.set(display.show_breakdown);
        self.breakdown.set_choices(
            spend_breakdown_choices(self.lang, Some(&state.spend)),
            &display.spend_breakdown,
        );
        for row in [
            self.period.row.upcast_ref::<gtk::Widget>(),
            self.unit.row.upcast_ref(),
            self.show_breakdown.row.upcast_ref(),
            self.breakdown.row.upcast_ref(),
        ] {
            row.set_sensitive(display.show_spend);
        }
    }
}
