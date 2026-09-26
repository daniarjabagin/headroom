use std::cell::RefCell;
use std::rc::Rc;

use adw::prelude::*;

use super::rows::{action_row, suffix_button};
use super::{Act, PrefsAction};
use crate::i18n::Lang;
use crate::payload::Account;
use crate::preferences::accounts::{LinkItem, Moves};
use crate::preferences::choices::removal_subtitle;

const LINK_ICON: &str = "adw-external-link-symbolic";
const ALERT_ICON: &str = "dialog-warning-symbolic";

pub type Mover = Rc<dyn Fn(&str, isize)>;
pub type Remover = Rc<dyn Fn(&Account)>;
pub type SignIn = Rc<dyn Fn(&str, &str)>;

#[derive(Clone)]
pub struct Handlers {
    pub act: Act,
    pub mover: Mover,
    pub remover: Remover,
    pub sign_in: SignIn,
}

pub struct SignInGroup {
    pub group: adw::PreferencesGroup,
    row: adw::ActionRow,
}

impl SignInGroup {
    pub fn new(lang: Lang, handlers: &Handlers, account: &Rc<RefCell<Account>>) -> Self {
        let row = action_row(
            "",
            lang.tr("Sign in again to keep this account up to date."),
        );
        let button = suffix_button(lang.tr("Sign in again…"), &["suggested-action"]);
        let (sign_in, account) = (Rc::clone(&handlers.sign_in), Rc::clone(account));
        button.connect_clicked(move |_| {
            let (provider, id) = {
                let account = account.borrow();
                (account.provider.clone(), account.id.clone())
            };
            sign_in(&provider, &id);
        });
        row.add_suffix(&button);
        let group = adw::PreferencesGroup::new();
        group.add(&row);
        Self { group, row }
    }

    pub fn update(&self, title: &str, offered: bool) {
        self.row.set_title(title);
        self.group.set_visible(offered);
    }
}

fn link_row(lang: Lang, act: &Act, link: &LinkItem) -> adw::ActionRow {
    let row = action_row(link.title, &link.subtitle);
    row.set_activatable(true);
    row.set_tooltip_text(Some(&link.url));
    if link.alert {
        let alert = gtk::Image::from_icon_name(ALERT_ICON);
        alert.add_css_class("warning");
        alert.set_tooltip_text(Some(lang.tr("Status page")));
        row.add_prefix(&alert);
    }
    row.add_suffix(&gtk::Image::from_icon_name(LINK_ICON));
    let (act, url) = (Rc::clone(act), link.url.clone());
    row.connect_activated(move |_| act(PrefsAction::OpenUrl(url.clone())));
    row
}

pub struct LinksGroup {
    pub group: adw::PreferencesGroup,
    rows: RefCell<Vec<adw::ActionRow>>,
    links: RefCell<Vec<LinkItem>>,
    lang: Lang,
    act: Act,
}

impl LinksGroup {
    pub fn new(lang: Lang, act: &Act) -> Self {
        Self {
            group: adw::PreferencesGroup::builder()
                .title(lang.tr("Links"))
                .visible(false)
                .build(),
            rows: RefCell::default(),
            links: RefCell::default(),
            lang,
            act: Rc::clone(act),
        }
    }

    pub fn update(&self, links: Vec<LinkItem>) {
        if *self.links.borrow() == links {
            return;
        }
        for row in self.rows.take() {
            self.group.remove(&row);
        }
        let rows: Vec<adw::ActionRow> = links
            .iter()
            .map(|link| link_row(self.lang, &self.act, link))
            .collect();
        for row in &rows {
            self.group.add(row);
        }
        self.group.set_visible(!rows.is_empty());
        *self.rows.borrow_mut() = rows;
        *self.links.borrow_mut() = links;
    }
}

pub struct ManageGroup {
    pub group: adw::PreferencesGroup,
    up: gtk::Button,
    down: gtk::Button,
}

fn move_button(icon: &str, tooltip: &str, mover: &Mover, id: &str, delta: isize) -> gtk::Button {
    let button = gtk::Button::from_icon_name(icon);
    button.set_tooltip_text(Some(tooltip));
    button.set_valign(gtk::Align::Center);
    let (mover, id) = (Rc::clone(mover), id.to_owned());
    button.connect_clicked(move |_| mover(&id, delta));
    button
}

fn remove_row(lang: Lang, remover: &Remover, account: &Rc<RefCell<Account>>) -> adw::ActionRow {
    let owner = account.borrow().owner;
    let row = action_row(
        lang.tr("Remove from Headroom"),
        removal_subtitle(lang, owner),
    );
    let button = suffix_button(lang.tr("Remove…"), &["destructive-action"]);
    let (remover, account) = (Rc::clone(remover), Rc::clone(account));
    button.connect_clicked(move |_| {
        let account = account.borrow().clone();
        remover(&account);
    });
    row.add_suffix(&button);
    row
}

impl ManageGroup {
    pub fn new(lang: Lang, handlers: &Handlers, account: &Rc<RefCell<Account>>) -> Self {
        let id = account.borrow().id.clone();
        let up = move_button(
            "go-up-symbolic",
            lang.tr("Move up"),
            &handlers.mover,
            &id,
            -1,
        );
        let down = move_button(
            "go-down-symbolic",
            lang.tr("Move down"),
            &handlers.mover,
            &id,
            1,
        );
        let buttons = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        buttons.add_css_class("linked");
        buttons.set_valign(gtk::Align::Center);
        buttons.append(&up);
        buttons.append(&down);
        let position = action_row(lang.tr("Position"), lang.tr("Order in the tray and popup"));
        position.add_suffix(&buttons);
        let group = adw::PreferencesGroup::new();
        group.add(&position);
        group.add(&remove_row(lang, &handlers.remover, account));
        Self { group, up, down }
    }

    pub fn update(&self, moves: Moves) {
        self.up.set_sensitive(moves.up);
        self.down.set_sensitive(moves.down);
    }
}
