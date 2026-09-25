use std::cell::RefCell;
use std::rc::Rc;

use adw::prelude::*;

use super::account_row::provider_image;
use super::picks::{LimitItem, limit_items, limits_summary};
use super::rows::Guard;
use super::{Act, PrefsAction};
use crate::i18n::Lang;
use crate::payload::{Display, PanelLimit, State};
use crate::preferences::change::Change;

const LOGO_SIZE: i32 = 24;

struct LimitRow {
    limit: PanelLimit,
    row: adw::ActionRow,
    check: gtk::CheckButton,
}

pub struct LimitsRow {
    pub row: adw::ExpanderRow,
    lang: Lang,
    act: Act,
    logo_color: String,
    rows: RefCell<Vec<LimitRow>>,
    shape: RefCell<Vec<(PanelLimit, String, String)>>,
    display: Rc<RefCell<Display>>,
    guard: Guard,
}

fn shape(items: &[LimitItem]) -> Vec<(PanelLimit, String, String)> {
    items
        .iter()
        .map(|item| {
            (
                item.limit.clone(),
                item.provider.clone(),
                item.title.clone(),
            )
        })
        .collect()
}

impl LimitsRow {
    pub fn new(lang: Lang, act: &Act, logo_color: &str) -> Self {
        let row = adw::ExpanderRow::builder()
            .title(lang.tr("Limits in the icon"))
            .use_markup(false)
            .build();
        Self {
            row,
            lang,
            act: Rc::clone(act),
            logo_color: logo_color.to_owned(),
            rows: RefCell::default(),
            shape: RefCell::default(),
            display: Rc::default(),
            guard: Guard::default(),
        }
    }

    pub fn update(&self, state: &State, display: &Display) {
        *self.display.borrow_mut() = display.clone();
        let items = limit_items(self.lang, state, display);
        let shape = shape(&items);
        if *self.shape.borrow() != shape {
            self.rebuild(&items);
            *self.shape.borrow_mut() = shape;
        }
        self.row
            .set_subtitle(&limits_summary(self.lang, display.panel_limits.len()));
        let room = display.can_add_panel_limit();
        self.guard.quietly(|| {
            for (row, item) in self.rows.borrow().iter().zip(&items) {
                let chosen = display.has_panel_limit(&row.limit);
                row.row.set_subtitle(&item.subtitle);
                if row.check.is_active() != chosen {
                    row.check.set_active(chosen);
                }
                row.row.set_sensitive(chosen || room);
            }
        });
    }

    fn rebuild(&self, items: &[LimitItem]) {
        for old in self.rows.take() {
            self.row.remove(&old.row);
        }
        let rows: Vec<LimitRow> = items.iter().map(|item| self.new_row(item)).collect();
        for row in &rows {
            self.row.add_row(&row.row);
        }
        *self.rows.borrow_mut() = rows;
    }

    fn new_row(&self, item: &LimitItem) -> LimitRow {
        let row = adw::ActionRow::builder()
            .title(&item.title)
            .use_markup(false)
            .activatable(true)
            .build();
        let check = gtk::CheckButton::new();
        check.set_valign(gtk::Align::Center);
        row.add_prefix(&provider_image(&item.provider, &self.logo_color, LOGO_SIZE));
        row.add_prefix(&check);
        row.set_activatable_widget(Some(&check));
        let (act, display, watch) = (
            Rc::clone(&self.act),
            Rc::clone(&self.display),
            self.guard.clone(),
        );
        let limit = item.limit.clone();
        check.connect_toggled(move |check| {
            if !watch.active() {
                return;
            }
            let limits = display
                .borrow()
                .panel_limits_after(&limit, check.is_active());
            act(PrefsAction::Change(Change::PanelLimits(limits)));
        });
        LimitRow {
            limit: item.limit.clone(),
            row,
            check,
        }
    }
}
