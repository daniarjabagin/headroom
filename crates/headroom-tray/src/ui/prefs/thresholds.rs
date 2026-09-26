use std::cell::RefCell;
use std::rc::Rc;

use adw::prelude::*;

use super::account_list::provider_image;
use super::picks::{ProviderItem, account_providers, thresholds_summary};
use super::rows::{ComboRow, SegmentedRow, changer};
use super::{Act, PrefsAction};
use crate::i18n::Lang;
use crate::payload::State;
use crate::preferences::change::Change;
use crate::preferences::choices::Choice;
use crate::preferences::model::Notifications;
use crate::preferences::options::provider_threshold_choices;

const PRESETS: [u8; 4] = [5, 10, 20, 30];
const LOGO_SIZE: i32 = 24;

pub struct ThresholdGroup {
    pub group: adw::PreferencesGroup,
    general: SegmentedRow<u8>,
    providers: adw::ExpanderRow,
    rows: RefCell<Vec<(ProviderItem, ComboRow<Option<u8>>)>>,
    lang: Lang,
    act: Act,
    logo_color: String,
}

fn percent_choices(current: u8) -> Vec<Choice<u8>> {
    let mut values = PRESETS.to_vec();
    if !values.contains(&current) {
        values.push(current);
        values.sort_unstable();
    }
    values
        .into_iter()
        .map(|value| Choice {
            value,
            label: format!("{value}%"),
        })
        .collect()
}

impl ThresholdGroup {
    pub fn new(lang: Lang, act: &Act, logo_color: &str) -> Self {
        let group = adw::PreferencesGroup::builder()
            .title(lang.tr("Alert Threshold"))
            .build();
        let general = SegmentedRow::new(
            lang.tr("Alert when less than"),
            lang.tr("Used by Almost out"),
            percent_choices(PRESETS[1]),
            changer(act, Change::ThresholdPercent),
        );
        let providers = adw::ExpanderRow::builder()
            .title(lang.tr("Per provider"))
            .use_markup(false)
            .build();
        group.add(&general.row);
        group.add(&providers);
        Self {
            group,
            general,
            providers,
            rows: RefCell::default(),
            lang,
            act: Rc::clone(act),
            logo_color: logo_color.to_owned(),
        }
    }

    pub fn update(&self, notifications: &Notifications, state: &State) {
        let general = notifications.threshold_percent;
        self.general.set_choices(percent_choices(general), &general);
        let providers = account_providers(state);
        let known: Vec<ProviderItem> = self
            .rows
            .borrow()
            .iter()
            .map(|(item, _)| item.clone())
            .collect();
        if known != providers {
            self.rebuild(&providers);
        }
        self.providers
            .set_subtitle(&thresholds_summary(self.lang, notifications, &providers));
        self.providers.set_visible(!providers.is_empty());
        for (item, row) in self.rows.borrow().iter() {
            let current = notifications.provider_thresholds.get(&item.id).copied();
            row.set(
                provider_threshold_choices(self.lang, general, current),
                &current,
            );
        }
    }

    fn rebuild(&self, providers: &[ProviderItem]) {
        for (_, old) in self.rows.take() {
            self.providers.remove(&old.row);
        }
        let rows: Vec<(ProviderItem, ComboRow<Option<u8>>)> = providers
            .iter()
            .map(|item| (item.clone(), self.new_row(item)))
            .collect();
        for (_, row) in &rows {
            self.providers.add_row(&row.row);
        }
        *self.rows.borrow_mut() = rows;
    }

    fn new_row(&self, item: &ProviderItem) -> ComboRow<Option<u8>> {
        let (act, provider) = (Rc::clone(&self.act), item.id.clone());
        let row = ComboRow::new(&item.name, "", move |threshold| {
            act(PrefsAction::Change(Change::ProviderThreshold {
                provider: provider.clone(),
                threshold,
            }));
        });
        row.row
            .add_prefix(&provider_image(&item.id, &self.logo_color, LOGO_SIZE));
        row
    }
}
