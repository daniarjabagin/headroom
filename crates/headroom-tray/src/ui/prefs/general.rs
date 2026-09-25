use adw::prelude::*;

use super::cards_group::CardsGroup;
use super::panel_group::PanelGroup;
use super::privacy_group::PrivacyGroup;
use super::rows::{ComboRow, SegmentedRow, SwitchRow, changer, group};
use super::spend_group::SpendGroup;
use super::{Act, Snapshot};
use crate::i18n::Lang;
use crate::payload::{Density, Language, ResetFormat, Theme, TimeFormat, ValueMode};
use crate::preferences::capability::Capabilities;
use crate::preferences::change::{Change, Section};
use crate::preferences::choices::{Choice, refresh_choices};
use crate::preferences::model::Settings;
use crate::preferences::options::{density_choices, time_format_choices};

struct Look {
    theme: SegmentedRow<Theme>,
    language: SegmentedRow<Language>,
    time_format: SegmentedRow<TimeFormat>,
    density: SegmentedRow<Density>,
    reduced_motion: SwitchRow,
    value_mode: SegmentedRow<ValueMode>,
    reset_format: SegmentedRow<ResetFormat>,
    combine: SwitchRow,
}

struct Refresh {
    interval: ComboRow<u32>,
    adaptive: SwitchRow,
}

pub struct GeneralPage {
    pub page: adw::PreferencesPage,
    look: Look,
    panel: PanelGroup,
    spend: SpendGroup,
    sections: Vec<(Section, SwitchRow)>,
    cards: CardsGroup,
    refresh: Refresh,
    privacy: PrivacyGroup,
}

fn choice<T>(value: T, label: &str) -> Choice<T> {
    Choice {
        value,
        label: label.to_owned(),
    }
}

fn section_text(lang: Lang, section: Section) -> (&'static str, &'static str) {
    match section {
        Section::Spend => (
            lang.tr("Total spend"),
            lang.tr("Spend ring for all tools at the top"),
        ),
        Section::AccountSpend => (
            lang.tr("Per-account spend"),
            lang.tr("Today, yesterday and 30 days under each account"),
        ),
        Section::Trend => (
            lang.tr("Usage trend"),
            lang.tr("Daily token bars for the last 30 days"),
        ),
        Section::Forecast => (
            lang.tr("Pace forecast"),
            lang.tr("Where each limit lands at the current pace"),
        ),
    }
}

fn section_value(settings: &Settings, section: Section) -> bool {
    let display = &settings.display;
    match section {
        Section::Spend => display.show_spend,
        Section::AccountSpend => display.show_account_spend,
        Section::Trend => display.show_trend,
        Section::Forecast => display.show_forecast,
    }
}

fn theme_row(lang: Lang, act: &Act) -> SegmentedRow<Theme> {
    SegmentedRow::new(
        lang.tr("Theme"),
        "",
        vec![
            choice(Theme::System, lang.tr("System")),
            choice(Theme::Light, lang.tr("Light")),
            choice(Theme::Dark, lang.tr("Dark")),
        ],
        changer(act, Change::Theme),
    )
}

fn language_row(lang: Lang, act: &Act) -> SegmentedRow<Language> {
    SegmentedRow::new(
        lang.tr("Language"),
        "",
        vec![
            choice(Language::System, lang.tr("System")),
            choice(Language::En, "English"),
            choice(Language::Ru, "Русский"),
        ],
        changer(act, Change::Language),
    )
}

fn value_mode_row(lang: Lang, act: &Act) -> SegmentedRow<ValueMode> {
    SegmentedRow::new(
        lang.tr("Show values as"),
        lang.tr("Click a reading in the popup to switch"),
        vec![
            choice(ValueMode::Left, lang.tr("Left")),
            choice(ValueMode::Used, lang.tr("Used")),
        ],
        changer(act, Change::ValueMode),
    )
}

fn reset_format_row(lang: Lang, act: &Act) -> SegmentedRow<ResetFormat> {
    SegmentedRow::new(
        lang.tr("Reset time"),
        lang.tr("Click a reset time in the popup to switch"),
        vec![
            choice(ResetFormat::Countdown, lang.tr("Countdown")),
            choice(ResetFormat::Exact, lang.tr("Exact time")),
        ],
        changer(act, Change::ResetFormat),
    )
}

fn appearance_rows(lang: Lang, act: &Act) -> Look {
    Look {
        theme: theme_row(lang, act),
        language: language_row(lang, act),
        time_format: SegmentedRow::new(
            lang.tr("Time format"),
            lang.tr("Exact reset times and chart labels"),
            time_format_choices(lang),
            changer(act, Change::TimeFormat),
        ),
        density: SegmentedRow::new(
            lang.tr("Density"),
            lang.tr("Compact fits more accounts in the popup"),
            density_choices(lang),
            changer(act, Change::Density),
        ),
        reduced_motion: SwitchRow::new(
            lang.tr("Reduced motion"),
            lang.tr("Turn off animations in the popup"),
            changer(act, Change::ReducedMotion),
        ),
        value_mode: value_mode_row(lang, act),
        reset_format: reset_format_row(lang, act),
        combine: SwitchRow::new(
            lang.tr("Combine accounts of the same provider"),
            lang.tr("Show one card per provider and add up the limits of its accounts"),
            changer(act, Change::CombineAccounts),
        ),
    }
}

fn section_rows(lang: Lang, act: &Act) -> Vec<(Section, SwitchRow)> {
    Section::ALL
        .into_iter()
        .map(|section| {
            let (title, subtitle) = section_text(lang, section);
            let row = SwitchRow::new(
                title,
                subtitle,
                changer(act, move |on| Change::Section(section, on)),
            );
            (section, row)
        })
        .collect()
}

fn refresh_rows(lang: Lang, act: &Act) -> Refresh {
    Refresh {
        interval: ComboRow::new(
            lang.tr("Refresh interval"),
            lang.tr("How often the service asks each provider"),
            changer(act, Change::RefreshInterval),
        ),
        adaptive: SwitchRow::new(
            lang.tr("Faster while coding tools run"),
            lang.tr("Every minute while a coding tool is writing usage logs"),
            changer(act, Change::AdaptiveRefresh),
        ),
    }
}

impl GeneralPage {
    pub fn new(lang: Lang, act: &Act, logo_color: &str) -> Self {
        let page = adw::PreferencesPage::builder()
            .title(lang.tr("General"))
            .icon_name("preferences-system-symbolic")
            .build();
        let general = Self {
            page,
            look: appearance_rows(lang, act),
            panel: PanelGroup::new(lang, act, logo_color),
            spend: SpendGroup::new(lang, act),
            sections: section_rows(lang, act),
            cards: CardsGroup::new(lang, act, logo_color),
            refresh: refresh_rows(lang, act),
            privacy: PrivacyGroup::new(lang, act),
        };
        general.assemble(lang);
        general
    }

    fn assemble(&self, lang: Lang) {
        let look = &self.look;
        self.page.add(&group(
            lang.tr("Appearance"),
            "",
            &[
                look.theme.row.upcast_ref(),
                look.language.row.upcast_ref(),
                look.time_format.row.upcast_ref(),
                look.density.row.upcast_ref(),
                look.reduced_motion.row.upcast_ref(),
            ],
        ));
        self.page.add(&group(
            lang.tr("Popup"),
            "",
            &[
                look.value_mode.row.upcast_ref(),
                look.reset_format.row.upcast_ref(),
                look.combine.row.upcast_ref(),
            ],
        ));
        self.page.add(&self.panel.group);
        self.page.add(&self.spend.group);
        let sections: Vec<&gtk::Widget> = self
            .sections
            .iter()
            .map(|(_, row)| row.row.upcast_ref())
            .collect();
        self.page.add(&group(lang.tr("Sections"), "", &sections));
        self.page.add(&self.cards.group);
        self.page.add(&group(
            lang.tr("Data refresh"),
            "",
            &[
                self.refresh.interval.row.upcast_ref(),
                self.refresh.adaptive.row.upcast_ref(),
            ],
        ));
        self.page.add(&self.privacy.group);
        self.page.add(&self.privacy.keyboard);
    }

    pub fn update(&self, snapshot: &Snapshot) {
        let (Some(settings), Some(state)) = (snapshot.settings, snapshot.state) else {
            return;
        };
        let capable = Capabilities::of(Some(state)).release_0_6;
        self.update_look(settings, capable);
        self.panel.update(snapshot);
        self.spend.group.set_visible(capable);
        self.spend.update(settings, state);
        for (section, row) in &self.sections {
            row.row.set_visible(!capable || *section != Section::Spend);
            row.set(section_value(settings, *section));
        }
        self.cards.group.set_visible(capable);
        self.cards.update(&settings.display, state);
        self.refresh.interval.set(
            refresh_choices(snapshot.lang, settings.refresh_interval_secs),
            &settings.refresh_interval_secs,
        );
        self.refresh.adaptive.row.set_visible(capable);
        self.refresh.adaptive.set(settings.adaptive_refresh);
        self.privacy.update(snapshot);
    }

    fn update_look(&self, settings: &Settings, capable: bool) {
        let (look, display) = (&self.look, &settings.display);
        look.theme.set(&display.theme);
        look.language.set(&display.language);
        look.time_format.row.set_visible(capable);
        look.time_format.set(&display.time_format);
        look.density.row.set_visible(capable);
        look.density.set(&display.density);
        look.reduced_motion.set(settings.reduced_motion);
        look.value_mode.set(&display.value_mode);
        look.reset_format.set(&display.reset_format);
        look.combine.set(display.combine_accounts);
    }
}
