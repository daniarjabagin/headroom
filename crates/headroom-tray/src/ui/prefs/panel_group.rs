use adw::prelude::*;

use super::limits::LimitsRow;
use super::rows::{ComboRow, SegmentedRow, changer};
use super::{Act, Snapshot};
use crate::i18n::Lang;
use crate::payload::{PanelIndicator, PanelLabel, PanelMode};
use crate::preferences::capability::Capabilities;
use crate::preferences::change::Change;
use crate::preferences::choices::headline_choices;
use crate::preferences::model::Headline;
use crate::preferences::options::{
    panel_indicator_choices, panel_label_choices, panel_mode_choices,
};

pub struct PanelGroup {
    pub group: adw::PreferencesGroup,
    mode: ComboRow<PanelMode>,
    headline: ComboRow<Headline>,
    limits: LimitsRow,
    indicator: SegmentedRow<PanelIndicator>,
    label: SegmentedRow<PanelLabel>,
}

impl PanelGroup {
    pub fn new(lang: Lang, act: &Act, logo_color: &str) -> Self {
        let capable = Capabilities { release_0_6: true };
        let panel = Self {
            group: adw::PreferencesGroup::builder()
                .title(lang.tr("Tray Icon"))
                .build(),
            mode: ComboRow::new(
                lang.tr("Tray icon shows"),
                lang.tr("What the icon in the tray draws"),
                changer(act, Change::PanelMode),
            ),
            headline: ComboRow::new(
                lang.tr("Tray limit"),
                lang.tr("The limit the tray icon and its tooltip show"),
                changer(act, Change::Headline),
            ),
            limits: LimitsRow::new(lang, act, logo_color),
            indicator: SegmentedRow::new(
                lang.tr("Indicator style"),
                lang.tr("The mark in front of each figure"),
                panel_indicator_choices(lang),
                changer(act, Change::PanelIndicator),
            ),
            label: SegmentedRow::new(
                lang.tr("Label"),
                lang.tr("The text next to each mark"),
                panel_label_choices(lang, capable),
                changer(act, Change::PanelLabel),
            ),
        };
        for row in [
            panel.mode.row.upcast_ref::<gtk::Widget>(),
            panel.headline.row.upcast_ref(),
            panel.limits.row.upcast_ref(),
            panel.indicator.row.upcast_ref(),
            panel.label.row.upcast_ref(),
        ] {
            panel.group.add(row);
        }
        panel
    }

    pub fn update(&self, snapshot: &Snapshot) {
        let (Some(settings), Some(state)) = (snapshot.settings, snapshot.state) else {
            return;
        };
        let lang = snapshot.lang;
        let display = &settings.display;
        let capable = Capabilities::of(Some(state)).release_0_6;
        let mode = if capable {
            display.panel_mode
        } else {
            PanelMode::Headline
        };
        self.mode.row.set_visible(capable);
        self.mode.set(panel_mode_choices(lang), &display.panel_mode);
        self.headline.row.set_visible(mode == PanelMode::Headline);
        self.headline.set(
            headline_choices(lang, Some(state), &settings.headline),
            &settings.headline,
        );
        self.limits.row.set_visible(mode == PanelMode::Several);
        if mode == PanelMode::Several {
            self.limits.update(state, display);
        }
        let marks = capable && mode != PanelMode::Icon;
        self.indicator.row.set_visible(marks);
        self.indicator.set(&display.panel_indicator);
        self.label.row.set_visible(marks);
        self.label.set(&display.panel_label);
    }
}
