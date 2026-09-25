use std::cell::RefCell;
use std::rc::Rc;

use adw::prelude::*;

use super::account_row::{AccountRow, Mover, Remover, shape};
use super::{Act, PrefsAction, Snapshot};
use crate::i18n::{Lang, fill};
use crate::payload::Account;
use crate::preferences::choices::{account_name, moved_order, removal_body};

const REMOVE: &str = "remove";
const CANCEL: &str = "cancel";

type Layout = (String, Vec<(String, Vec<(String, String)>)>);

pub struct AccountsPage {
    pub page: adw::PreferencesPage,
    list: gtk::ListBox,
    rows: RefCell<Vec<AccountRow>>,
    layout: RefCell<Option<Layout>>,
    order: Rc<RefCell<Vec<String>>>,
    lang: Lang,
    act: Act,
    mover: Mover,
    remover: Remover,
}

fn placeholder(lang: Lang) -> adw::ActionRow {
    let row = adw::ActionRow::builder()
        .title(lang.tr("No accounts yet"))
        .subtitle(lang.tr("Sign in with a supported CLI, or add an account below."))
        .build();
    row.add_css_class("dim-label");
    row
}

fn add_group(lang: Lang, on_add: Rc<dyn Fn()>) -> adw::PreferencesGroup {
    let group = adw::PreferencesGroup::builder()
        .title(lang.tr("Add Account"))
        .description(
            lang.tr("Sign in through a CLI, paste an API key, or let Headroom find the account."),
        )
        .build();
    let row = adw::ActionRow::builder()
        .title(lang.tr("Add account…"))
        .activatable(true)
        .build();
    row.add_prefix(&gtk::Image::from_icon_name("list-add-symbolic"));
    row.add_suffix(&gtk::Image::from_icon_name("go-next-symbolic"));
    row.connect_activated(move |_| on_add());
    group.add(&row);
    group
}

fn mover(act: &Act, order: &Rc<RefCell<Vec<String>>>) -> Mover {
    let (act, order) = (Rc::clone(act), Rc::clone(order));
    Rc::new(move |id: &str, delta: isize| {
        let moved = moved_order(&order.borrow(), id, delta);
        if let Some(moved) = moved {
            act(PrefsAction::SetOrder(moved));
        }
    })
}

fn confirm_removal(lang: Lang, parent: &gtk::Widget, act: &Act, account: &Account) {
    let dialog = adw::AlertDialog::new(
        Some(&fill(
            lang.tr("Remove {name}?"),
            &[("name", &account_name(account))],
        )),
        Some(&removal_body(lang, account)),
    );
    dialog.add_responses(&[(CANCEL, lang.tr("Cancel")), (REMOVE, lang.tr("Remove"))]);
    dialog.set_response_appearance(REMOVE, adw::ResponseAppearance::Destructive);
    dialog.set_default_response(Some(CANCEL));
    dialog.set_close_response(CANCEL);
    let (act, account) = (Rc::clone(act), account.clone());
    dialog.connect_response(None, move |_, response| {
        if response == REMOVE {
            act(PrefsAction::Remove(Box::new(account.clone())));
        }
    });
    dialog.present(Some(parent));
}

fn remover(lang: Lang, act: &Act, page: &adw::PreferencesPage) -> Remover {
    let (act, page) = (Rc::clone(act), page.downgrade());
    Rc::new(move |account: &Account| {
        if let Some(page) = page.upgrade() {
            confirm_removal(lang, page.upcast_ref(), &act, account);
        }
    })
}

impl AccountsPage {
    pub fn new(lang: Lang, act: &Act, on_add: Rc<dyn Fn()>) -> Self {
        let page = adw::PreferencesPage::builder()
            .title(lang.tr("Accounts"))
            .icon_name("system-users-symbolic")
            .build();
        let list = gtk::ListBox::new();
        list.set_selection_mode(gtk::SelectionMode::None);
        list.add_css_class("boxed-list");
        list.set_placeholder(Some(&placeholder(lang)));
        let accounts = adw::PreferencesGroup::builder()
            .title(lang.tr("Accounts"))
            .description(
                lang.tr("Hidden accounts keep updating but leave the tray and notifications."),
            )
            .build();
        accounts.add(&list);
        page.add(&accounts);
        page.add(&add_group(lang, on_add));
        let order: Rc<RefCell<Vec<String>>> = Rc::default();
        Self {
            mover: mover(act, &order),
            remover: remover(lang, act, &page),
            page,
            list,
            rows: RefCell::default(),
            layout: RefCell::default(),
            order,
            lang,
            act: Rc::clone(act),
        }
    }

    pub fn update(&self, snapshot: &Snapshot) {
        let (Some(state), Some(settings)) = (snapshot.state, snapshot.settings) else {
            return;
        };
        let layout: Layout = (
            snapshot.logo_color.clone(),
            state
                .accounts
                .iter()
                .map(|account| (account.id.clone(), shape(account)))
                .collect(),
        );
        let current = self.layout.borrow().clone();
        if current.as_ref() != Some(&layout) {
            let same_color = current.is_some_and(|(color, _)| color == layout.0);
            self.rebuild(&state.accounts, &snapshot.logo_color, same_color);
            *self.layout.borrow_mut() = Some(layout);
        }
        for (row, account) in self.rows.borrow().iter().zip(&state.accounts) {
            row.update(self.lang, account, &settings.display);
        }
    }

    pub fn focus(&self, account_id: &str) {
        if let Some(row) = self.rows.borrow().iter().find(|row| row.id == account_id) {
            row.widget.set_expanded(true);
            row.widget.grab_focus();
        }
    }

    fn rebuild(&self, accounts: &[Account], logo_color: &str, reuse: bool) {
        let mut previous = self.rows.take();
        self.list.remove_all();
        let rows: Vec<AccountRow> = accounts
            .iter()
            .map(|account| {
                let kept = previous
                    .iter()
                    .position(|row| reuse && row.id == account.id && row.shape == shape(account));
                let row = match kept {
                    Some(index) => previous.swap_remove(index),
                    None => self.new_row(account, logo_color),
                };
                self.list.append(&row.widget);
                row
            })
            .collect();
        *self.rows.borrow_mut() = rows;
        *self.order.borrow_mut() = accounts.iter().map(|account| account.id.clone()).collect();
    }

    fn new_row(&self, account: &Account, logo_color: &str) -> AccountRow {
        let row = AccountRow::new(self.lang, account, logo_color);
        row.assemble(self.lang, &self.act, &self.mover, &self.remover);
        row
    }
}
