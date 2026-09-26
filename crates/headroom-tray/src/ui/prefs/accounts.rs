use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

use adw::prelude::*;

use super::account_actions::{Handlers, Mover, Remover, SignIn};
use super::account_detail::{AccountDetail, DetailInput, shape};
use super::account_list::SidebarRow;
use super::account_panes::{DETAIL, EMPTY, Panes};
use super::{Act, PrefsAction, Snapshot};
use crate::i18n::{Lang, fill};
use crate::payload::{Account, Display, ProviderStatus};
use crate::preferences::accounts::{
    SidebarItem, account_links, account_moves, kept_selection, sidebar_items, sidebar_title,
};
use crate::preferences::choices::{account_name, moved_order, removal_body};
use crate::preferences::registry::ProviderInfo;

const REMOVE: &str = "remove";
const CANCEL: &str = "cancel";

#[derive(Default)]
struct Data {
    accounts: Vec<Account>,
    display: Display,
    providers: Vec<ProviderInfo>,
    statuses: Vec<ProviderStatus>,
    logo_color: String,
}

pub struct AccountsPage {
    panes: Panes,
    rows: RefCell<Vec<SidebarRow>>,
    detail: RefCell<Option<(String, AccountDetail)>>,
    selected: RefCell<Option<String>>,
    data: RefCell<Data>,
    syncing: Cell<bool>,
    handlers: Handlers,
    lang: Lang,
}

fn mover(act: &Act, order: Weak<AccountsPage>) -> Mover {
    let act = Rc::clone(act);
    Rc::new(move |id: &str, delta: isize| {
        let Some(page) = order.upgrade() else {
            return;
        };
        let moved = moved_order(&page.order(), id, delta);
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

fn remover(lang: Lang, act: &Act, page: Weak<AccountsPage>) -> Remover {
    let act = Rc::clone(act);
    Rc::new(move |account: &Account| {
        if let Some(page) = page.upgrade() {
            confirm_removal(lang, page.panes.root.upcast_ref(), &act, account);
        }
    })
}

fn statuses(snapshot: &Snapshot) -> Vec<ProviderStatus> {
    snapshot
        .state
        .map(|state| state.provider_status.clone())
        .unwrap_or_default()
}

fn providers(snapshot: &Snapshot) -> Vec<ProviderInfo> {
    snapshot
        .providers
        .and_then(|providers| providers.as_ref().ok())
        .cloned()
        .unwrap_or_default()
}

impl AccountsPage {
    pub fn new(lang: Lang, act: &Act, on_add: &Rc<dyn Fn()>, sign_in: SignIn) -> Rc<Self> {
        Rc::new_cyclic(|weak: &Weak<Self>| {
            let page = Self {
                panes: Panes::new(lang, on_add),
                rows: RefCell::default(),
                detail: RefCell::default(),
                selected: RefCell::default(),
                data: RefCell::default(),
                syncing: Cell::new(false),
                handlers: Handlers {
                    act: Rc::clone(act),
                    mover: mover(act, weak.clone()),
                    remover: remover(lang, act, weak.clone()),
                    sign_in,
                },
                lang,
            };
            page.connect_list(weak.clone());
            page
        })
    }

    #[must_use]
    pub fn widget(&self) -> gtk::Widget {
        self.panes.root.clone().upcast()
    }

    fn connect_list(&self, weak: Weak<Self>) {
        let selecting = weak.clone();
        self.panes.list.connect_row_selected(move |_, row| {
            if let Some(page) = selecting.upgrade() {
                page.selected_row(row);
            }
        });
        self.panes.list.connect_row_activated(move |_, _| {
            if let Some(page) = weak.upgrade() {
                page.panes.split.set_show_content(true);
            }
        });
    }

    fn order(&self) -> Vec<String> {
        self.data
            .borrow()
            .accounts
            .iter()
            .map(|account| account.id.clone())
            .collect()
    }

    fn selected_row(&self, row: Option<&gtk::ListBoxRow>) {
        if self.syncing.get() {
            return;
        }
        let id = row.and_then(|row| {
            self.rows
                .borrow()
                .iter()
                .find(|known| known.row.upcast_ref::<gtk::ListBoxRow>() == row)
                .map(|known| known.account_id.clone())
        });
        match id {
            Some(id) => {
                *self.selected.borrow_mut() = Some(id);
                self.render_detail();
            }
            None => self.select_current_row(),
        }
    }

    pub fn update(&self, snapshot: &Snapshot) {
        let (Some(state), Some(settings)) = (snapshot.state, snapshot.settings) else {
            return;
        };
        let previous = self.order();
        *self.data.borrow_mut() = Data {
            accounts: state.accounts.clone(),
            display: settings.display.clone(),
            providers: providers(snapshot),
            statuses: statuses(snapshot),
            logo_color: snapshot.logo_color.clone(),
        };
        let items = {
            let data = self.data.borrow();
            sidebar_items(self.lang, &data.accounts, &data.display)
        };
        let kept = kept_selection(&items, &previous, self.selected.borrow().as_deref());
        *self.selected.borrow_mut() = kept;
        self.sync_rows(&items);
        self.render_detail();
    }

    pub fn focus(&self, account_id: &str) {
        *self.selected.borrow_mut() = Some(account_id.to_owned());
        self.select_current_row();
        self.render_detail();
        self.panes.split.set_show_content(true);
    }

    fn sync_rows(&self, items: &[SidebarItem]) {
        let color = self.data.borrow().logo_color.clone();
        let fits = {
            let rows = self.rows.borrow();
            rows.len() == items.len() && rows.iter().zip(items).all(|(row, item)| row.fits(item))
        };
        if !fits {
            self.syncing.set(true);
            self.panes.list.remove_all();
            let rows: Vec<SidebarRow> = items
                .iter()
                .map(|item| SidebarRow::new(item, &color))
                .collect();
            for row in &rows {
                self.panes.list.append(&row.row);
            }
            *self.rows.borrow_mut() = rows;
            self.syncing.set(false);
        }
        for (row, item) in self.rows.borrow().iter().zip(items) {
            row.update(self.lang, item);
        }
        self.select_current_row();
    }

    fn select_current_row(&self) {
        let selected = self.selected.borrow().clone();
        let rows = self.rows.borrow();
        let row = rows
            .iter()
            .find(|row| Some(&row.account_id) == selected.as_ref());
        self.syncing.set(true);
        match row {
            Some(row) => self.panes.list.select_row(Some(&row.row)),
            None => self.panes.list.unselect_all(),
        }
        self.syncing.set(false);
    }

    fn render_detail(&self) {
        let data = self.data.borrow();
        let selected = self.selected.borrow().clone();
        let account = selected
            .as_deref()
            .and_then(|id| data.accounts.iter().find(|account| account.id == id));
        let Some(account) = account else {
            self.panes.stack.set_visible_child_name(EMPTY);
            self.panes.content.set_title(self.lang.tr("Accounts"));
            return;
        };
        self.ensure_detail(account, &data.logo_color);
        let status = data
            .statuses
            .iter()
            .find(|status| status.provider == account.provider);
        let provider = data
            .providers
            .iter()
            .find(|provider| provider.id == account.provider);
        let input = DetailInput {
            account,
            accounts: &data.accounts,
            display: &data.display,
            moves: account_moves(&self.order(), &account.id),
            links: account_links(self.lang, provider, status),
        };
        if let Some((_, detail)) = self.detail.borrow().as_ref() {
            detail.update(input);
        }
        self.panes
            .content
            .set_title(&sidebar_title(account, &data.accounts));
        self.panes.stack.set_visible_child_name(DETAIL);
    }

    fn ensure_detail(&self, account: &Account, logo_color: &str) {
        let current = self
            .detail
            .borrow()
            .as_ref()
            .is_some_and(|(color, detail)| {
                color == logo_color && detail.id == account.id && detail.shape == shape(account)
            });
        if current {
            return;
        }
        if let Some(old) = self.panes.stack.child_by_name(DETAIL) {
            self.panes.stack.remove(&old);
        }
        let detail = AccountDetail::new(self.lang, &self.handlers, account, logo_color);
        self.panes.stack.add_named(&detail.page, Some(DETAIL));
        *self.detail.borrow_mut() = Some((logo_color.to_owned(), detail));
    }
}
