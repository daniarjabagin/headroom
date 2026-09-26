use std::cell::RefCell;
use std::rc::Rc;

use adw::prelude::*;

use super::account_actions::{Handlers, LinksGroup, ManageGroup, SignInGroup};
use super::account_list::provider_image;
use super::rows::SwitchRow;
use super::{Act, PrefsAction};
use crate::format::{percent_reading, window_label};
use crate::i18n::Lang;
use crate::payload::{Account, Display, ValueMode};
use crate::preferences::accounts::{LinkItem, Moves, offers_sign_in, sidebar_title};
use crate::preferences::change::Change;
use crate::preferences::choices::account_subtitle;

const HEADER_LOGO: i32 = 32;
const HEADER_SPACING: i32 = 12;

pub struct DetailInput<'a> {
    pub account: &'a Account,
    pub accounts: &'a [Account],
    pub display: &'a Display,
    pub moves: Moves,
    pub links: Vec<LinkItem>,
}

pub struct AccountDetail {
    pub page: adw::PreferencesPage,
    pub id: String,
    pub shape: Vec<(String, String)>,
    title: gtk::Label,
    subtitle: gtk::Label,
    visible: SwitchRow,
    star: SwitchRow,
    label: adw::EntryRow,
    windows: Vec<(String, SwitchRow)>,
    sign_in: SignInGroup,
    links: LinksGroup,
    manage: ManageGroup,
    display: Rc<RefCell<Display>>,
    account: Rc<RefCell<Account>>,
    lang: Lang,
}

#[must_use]
pub fn shape(account: &Account) -> Vec<(String, String)> {
    let head = (format!("{:?}", account.owner), account.provider.clone());
    std::iter::once(head)
        .chain(
            account
                .windows
                .iter()
                .map(|window| (window.id.clone(), window.label.clone())),
        )
        .collect()
}

fn header(account: &Account, logo_color: &str) -> (gtk::Box, gtk::Label, gtk::Label) {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, HEADER_SPACING);
    row.append(&provider_image(&account.provider, logo_color, HEADER_LOGO));
    let texts = gtk::Box::new(gtk::Orientation::Vertical, 2);
    texts.set_valign(gtk::Align::Center);
    let title = gtk::Label::builder().xalign(0.0).wrap(true).build();
    title.add_css_class("title-3");
    let subtitle = gtk::Label::builder().xalign(0.0).wrap(true).build();
    subtitle.add_css_class("dim-label");
    subtitle.add_css_class("caption");
    texts.append(&title);
    texts.append(&subtitle);
    row.append(&texts);
    (row, title, subtitle)
}

fn visibility_row(lang: Lang, act: &Act, id: &str) -> SwitchRow {
    let (act, id) = (Rc::clone(act), id.to_owned());
    SwitchRow::new(lang.tr("Show in the tray and popup"), "", move |shown| {
        act(PrefsAction::SetHidden {
            account_id: id.clone(),
            hidden: !shown,
        });
    })
}

fn star_row(lang: Lang, act: &Act, id: &str, display: &Rc<RefCell<Display>>) -> SwitchRow {
    let (act, id, display) = (Rc::clone(act), id.to_owned(), Rc::clone(display));
    let row = SwitchRow::new(
        lang.tr("Always open in the popup"),
        lang.tr("Stays expanded when unstarred accounts collapse"),
        move |starred| {
            let ids = display.borrow().starred_after(&id, starred);
            act(PrefsAction::Change(Change::StarredAccounts(ids)));
        },
    );
    row.row
        .add_prefix(&gtk::Image::from_icon_name("starred-symbolic"));
    row
}

fn label_row(lang: Lang, act: &Act, id: &str) -> adw::EntryRow {
    let row = adw::EntryRow::builder()
        .title(lang.tr("Label"))
        .show_apply_button(true)
        .build();
    let (act, id) = (Rc::clone(act), id.to_owned());
    row.connect_apply(move |entry| {
        act(PrefsAction::SetLabel {
            account_id: id.clone(),
            label: entry.text().trim().to_owned(),
        });
    });
    row
}

fn window_row(act: &Act, id: &str, window: &str, display: &Rc<RefCell<Display>>) -> SwitchRow {
    let (act, id, window) = (Rc::clone(act), id.to_owned(), window.to_owned());
    let display = Rc::clone(display);
    SwitchRow::new("", "", move |shown| {
        let windows = display.borrow().hidden_windows_after(&id, &window, !shown);
        act(PrefsAction::Change(Change::HiddenWindows {
            account_id: id.clone(),
            windows,
        }));
    })
}

impl AccountDetail {
    pub fn new(lang: Lang, handlers: &Handlers, account: &Account, logo_color: &str) -> Self {
        let act = &handlers.act;
        let display: Rc<RefCell<Display>> = Rc::default();
        let shared = Rc::new(RefCell::new(account.clone()));
        let (head, title, subtitle) = header(account, logo_color);
        let detail = Self {
            page: adw::PreferencesPage::new(),
            id: account.id.clone(),
            shape: shape(account),
            title,
            subtitle,
            visible: visibility_row(lang, act, &account.id),
            star: star_row(lang, act, &account.id, &display),
            label: label_row(lang, act, &account.id),
            windows: account
                .windows
                .iter()
                .map(|window| {
                    let row = window_row(act, &account.id, &window.id, &display);
                    (window.id.clone(), row)
                })
                .collect(),
            sign_in: SignInGroup::new(lang, handlers, &shared),
            links: LinksGroup::new(lang, act),
            manage: ManageGroup::new(lang, handlers, &shared),
            display,
            account: shared,
            lang,
        };
        detail.assemble(&head);
        detail
    }

    fn assemble(&self, head: &gtk::Box) {
        let general = adw::PreferencesGroup::new();
        general.add(head);
        let rows = adw::PreferencesGroup::new();
        rows.add(&self.visible.row);
        rows.add(&self.star.row);
        rows.add(&self.label);
        self.page.add(&general);
        self.page.add(&rows);
        if !self.windows.is_empty() {
            let limits = adw::PreferencesGroup::builder()
                .title(self.lang.tr("Limits"))
                .description(
                    self.lang
                        .tr("Hidden limits leave the popup, the tray and notifications."),
                )
                .build();
            for (_, row) in &self.windows {
                limits.add(&row.row);
            }
            self.page.add(&limits);
        }
        self.page.add(&self.sign_in.group);
        self.page.add(&self.links.group);
        self.page.add(&self.manage.group);
    }

    pub fn update(&self, input: DetailInput) {
        let (account, display) = (input.account, input.display);
        let title = sidebar_title(account, input.accounts);
        self.title.set_text(&title);
        self.subtitle
            .set_text(&account_subtitle(self.lang, account));
        self.visible.set(!account.hidden);
        self.star.set(display.is_starred(&account.id));
        self.update_label(account);
        for (window_id, row) in &self.windows {
            if let Some(window) = account
                .windows
                .iter()
                .find(|window| window.id == *window_id)
            {
                row.row
                    .set_title(&window_label(self.lang, &window.id, &window.label));
                row.row.set_subtitle(&percent_reading(
                    self.lang,
                    window.remaining_percent,
                    ValueMode::Left,
                ));
            }
            row.set(!display.is_window_hidden(&account.id, window_id));
        }
        self.sign_in.update(&title, offers_sign_in(account));
        self.links.update(input.links);
        self.manage.update(input.moves);
        *self.display.borrow_mut() = display.clone();
        *self.account.borrow_mut() = account.clone();
    }

    fn update_label(&self, account: &Account) {
        let editing = self
            .label
            .state_flags()
            .contains(gtk::StateFlags::FOCUS_WITHIN);
        let label = account.label.as_deref().unwrap_or_default();
        if !editing && self.label.text() != label {
            self.label.set_text(label);
        }
    }
}
