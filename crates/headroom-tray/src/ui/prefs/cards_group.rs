use std::cell::RefCell;
use std::rc::Rc;

use adw::prelude::*;

use super::account_row::provider_image;
use super::picks::{StarItem, star_items, star_label};
use super::rows::{SwitchRow, action_row, changer, icon_button};
use super::{Act, PrefsAction};
use crate::i18n::Lang;
use crate::payload::{Display, State};
use crate::preferences::change::Change;

const LOGO_SIZE: i32 = 24;
const STARRED: &str = "starred-symbolic";
const UNSTARRED: &str = "non-starred-symbolic";

struct StarRow {
    account_id: String,
    row: adw::ActionRow,
    status: gtk::Label,
    star: gtk::Button,
}

pub struct CardsGroup {
    pub group: adw::PreferencesGroup,
    collapse: SwitchRow,
    rows: RefCell<Vec<StarRow>>,
    items: RefCell<Vec<StarItem>>,
    display: Rc<RefCell<Display>>,
    lang: Lang,
    act: Act,
    logo_color: String,
}

impl CardsGroup {
    pub fn new(lang: Lang, act: &Act, logo_color: &str) -> Self {
        let group = adw::PreferencesGroup::builder()
            .title(lang.tr("Popup Cards"))
            .description(lang.tr("Star the accounts that always show their limits."))
            .build();
        let collapse = SwitchRow::new(
            lang.tr("Collapse unstarred accounts"),
            lang.tr("They fold into one line in the popup and open on click"),
            changer(act, Change::CollapseUnstarred),
        );
        group.add(&collapse.row);
        Self {
            group,
            collapse,
            rows: RefCell::default(),
            items: RefCell::default(),
            display: Rc::default(),
            lang,
            act: Rc::clone(act),
            logo_color: logo_color.to_owned(),
        }
    }

    pub fn update(&self, display: &Display, state: &State) {
        *self.display.borrow_mut() = display.clone();
        self.collapse.set(display.collapse_unstarred);
        let items = star_items(state);
        if *self.items.borrow() != items {
            self.rebuild(&items);
            *self.items.borrow_mut() = items;
        }
        for row in self.rows.borrow().iter() {
            let starred = display.is_starred(&row.account_id);
            row.status.set_text(star_label(self.lang, starred));
            row.star
                .set_icon_name(if starred { STARRED } else { UNSTARRED });
        }
    }

    fn rebuild(&self, items: &[StarItem]) {
        for old in self.rows.take() {
            self.group.remove(&old.row);
        }
        let rows: Vec<StarRow> = items.iter().map(|item| self.new_row(item)).collect();
        for row in &rows {
            self.group.add(&row.row);
        }
        *self.rows.borrow_mut() = rows;
    }

    fn new_row(&self, item: &StarItem) -> StarRow {
        let row = action_row(&item.title, &item.subtitle);
        row.add_prefix(&provider_image(&item.provider, &self.logo_color, LOGO_SIZE));
        let status = gtk::Label::new(None);
        status.add_css_class("dim-label");
        let star = icon_button(UNSTARRED, self.lang.tr("Always open in the popup"));
        row.add_suffix(&status);
        row.add_suffix(&star);
        row.set_activatable_widget(Some(&star));
        let (act, display, id) = (
            Rc::clone(&self.act),
            Rc::clone(&self.display),
            item.account_id.clone(),
        );
        star.connect_clicked(move |_| {
            let display = display.borrow();
            let starred = display.starred_after(&id, !display.is_starred(&id));
            act(PrefsAction::Change(Change::StarredAccounts(starred)));
        });
        StarRow {
            account_id: item.account_id.clone(),
            row,
            status,
            star,
        }
    }
}
