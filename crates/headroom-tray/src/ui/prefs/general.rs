use std::rc::Rc;

use adw::prelude::*;

use super::rows::{ComboRow, SegmentedRow, SwitchRow, group};
use super::{Act, PrefsAction, Snapshot};
use crate::i18n::Lang;
use crate::payload::{Language, ResetFormat, Theme, ValueMode};
use crate::preferences::change::{Change, Section};
use crate::preferences::choices::{Choice, headline_choices, refresh_choices};
use crate::preferences::model::{Headline, Settings};

struct Look {
    theme: SegmentedRow<Theme>,
    language: SegmentedRow<Language>,
    value_mode: SegmentedRow<ValueMode>,
    reset_format: SegmentedRow<ResetFormat>,
    combine: SwitchRow,
    reduced_motion: SwitchRow,
}

pub struct GeneralPage {
    pub page: adw::PreferencesPage,
    look: Look,
    headline: ComboRow<Headline>,
    sections: Vec<(Section, SwitchRow)>,
    refresh: ComboRow<u32>,
    check_updates: SwitchRow,
}

fn choice<T>(value: T, label: &str) -> Choice<T> {
    Choice {
        value,
        label: label.to_owned(),
    }
}

fn changer<T: 'static>(act: &Act, change: impl Fn(T) -> Change + 'static) -> impl Fn(T) + 'static {
    let act = Rc::clone(act);
    move |value| act(PrefsAction::Change(change(value)))
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

fn appearance_rows(lang: Lang, act: &Act) -> (SegmentedRow<Theme>, SegmentedRow<Language>) {
    let theme = SegmentedRow::new(
        lang.tr("Theme"),
        "",
        vec![
            choice(Theme::System, lang.tr("System")),
            choice(Theme::Light, lang.tr("Light")),
            choice(Theme::Dark, lang.tr("Dark")),
        ],
        changer(act, Change::Theme),
    );
    let language = SegmentedRow::new(
        lang.tr("Language"),
        "",
        vec![
            choice(Language::System, lang.tr("System")),
            choice(Language::En, "English"),
            choice(Language::Ru, "Русский"),
        ],
        changer(act, Change::Language),
    );
    (theme, language)
}

fn look_rows(lang: Lang, act: &Act) -> Look {
    let (theme, language) = appearance_rows(lang, act);
    Look {
        theme,
        language,
        value_mode: SegmentedRow::new(
            lang.tr("Show values as"),
            lang.tr("Click a reading in the popup to switch"),
            vec![
                choice(ValueMode::Left, lang.tr("Left")),
                choice(ValueMode::Used, lang.tr("Used")),
            ],
            changer(act, Change::ValueMode),
        ),
        reset_format: SegmentedRow::new(
            lang.tr("Reset time"),
            lang.tr("Click a reset time in the popup to switch"),
            vec![
                choice(ResetFormat::Countdown, lang.tr("Countdown")),
                choice(ResetFormat::Exact, lang.tr("Exact time")),
            ],
            changer(act, Change::ResetFormat),
        ),
        combine: SwitchRow::new(
            lang.tr("Combine accounts of the same provider"),
            lang.tr("Show one card per provider and add up the limits of its accounts"),
            changer(act, Change::CombineAccounts),
        ),
        reduced_motion: SwitchRow::new(
            lang.tr("Reduced motion"),
            lang.tr("Turn off animations in the popup"),
            changer(act, Change::ReducedMotion),
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

impl GeneralPage {
    pub fn new(lang: Lang, act: &Act) -> Self {
        let page = adw::PreferencesPage::builder()
            .title(lang.tr("General"))
            .icon_name("preferences-system-symbolic")
            .build();
        let look = look_rows(lang, act);
        let headline = ComboRow::new(
            lang.tr("Tray limit"),
            lang.tr("The limit the tray icon and its tooltip show"),
            changer(act, Change::Headline),
        );
        let sections = section_rows(lang, act);
        let refresh = ComboRow::new(
            lang.tr("Refresh interval"),
            lang.tr("How often the service asks each provider"),
            changer(act, Change::RefreshInterval),
        );
        let check_updates = SwitchRow::new(
            lang.tr("Check for updates"),
            lang.tr("Once a day, asks GitHub for the latest release. Nothing else is sent."),
            changer(act, Change::CheckUpdates),
        );
        let general = Self {
            page,
            look,
            headline,
            sections,
            refresh,
            check_updates,
        };
        general.assemble(lang);
        general
    }

    fn assemble(&self, lang: Lang) {
        let look = &self.look;
        self.page.add(&group(
            lang.tr("Appearance"),
            "",
            &[look.theme.row.upcast_ref(), look.language.row.upcast_ref()],
        ));
        self.page.add(&group(
            lang.tr("Popup"),
            "",
            &[
                look.value_mode.row.upcast_ref(),
                look.reset_format.row.upcast_ref(),
                look.combine.row.upcast_ref(),
                look.reduced_motion.row.upcast_ref(),
            ],
        ));
        self.page.add(&group(
            lang.tr("Tray Icon"),
            "",
            &[self.headline.row.upcast_ref()],
        ));
        let sections: Vec<&gtk::Widget> = self
            .sections
            .iter()
            .map(|(_, row)| row.row.upcast_ref())
            .collect();
        self.page.add(&group(lang.tr("Sections"), "", &sections));
        self.page.add(&group(
            lang.tr("Data refresh"),
            "",
            &[self.refresh.row.upcast_ref()],
        ));
        self.page.add(&group(
            lang.tr("Updates"),
            "",
            &[self.check_updates.row.upcast_ref()],
        ));
    }

    pub fn update(&self, snapshot: &Snapshot) {
        let Some(settings) = snapshot.settings else {
            return;
        };
        let (look, display) = (&self.look, &settings.display);
        look.theme.set(&display.theme);
        look.language.set(&display.language);
        look.value_mode.set(&display.value_mode);
        look.reset_format.set(&display.reset_format);
        look.combine.set(display.combine_accounts);
        look.reduced_motion.set(settings.reduced_motion);
        for (section, row) in &self.sections {
            row.set(section_value(settings, *section));
        }
        let lang = snapshot.lang;
        self.headline.set(
            headline_choices(lang, snapshot.state, &settings.headline),
            &settings.headline,
        );
        self.refresh.set(
            refresh_choices(lang, settings.refresh_interval_secs),
            &settings.refresh_interval_secs,
        );
        self.check_updates.set(settings.updates.check);
    }
}
