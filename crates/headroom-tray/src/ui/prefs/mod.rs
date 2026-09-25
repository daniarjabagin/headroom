mod about;
mod account_row;
mod accounts;
mod add_account;
mod detect_page;
mod flow_page;
mod general;
mod key_page;
mod login_page;
mod notifications;
mod rows;
mod service;

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use adw::prelude::*;

use crate::i18n::Lang;
use crate::payload::{Account, State};
use crate::preferences::change::Change;
use crate::preferences::model::Settings;
use crate::preferences::registry::{ProviderInfo, RegistryError};
use crate::update::UpdateRun;
use crate::update_check::CheckRun;
use about::AboutPage;
use accounts::AccountsPage;
use add_account::{AddAccountDialog, DialogCtx};
use general::GeneralPage;
use notifications::NotificationsPage;
use service::ServicePage;

const DEFAULT_WIDTH: i32 = 760;
const DEFAULT_HEIGHT: i32 = 820;
const TOAST_SECONDS: u32 = 4;

#[derive(Debug, Clone, PartialEq)]
pub enum PrefsAction {
    Change(Change),
    SetLabel { account_id: String, label: String },
    SetHidden { account_id: String, hidden: bool },
    SetOrder(Vec<String>),
    Remove(Box<Account>),
    Restore(String),
    StartService,
    InstallUpdate,
    OpenUrl(String),
    CheckForUpdates,
}

pub type Act = Rc<dyn Fn(PrefsAction)>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Service {
    Connecting,
    Stopped {
        starting: bool,
        error: Option<String>,
    },
    Running,
}

pub struct Snapshot<'a> {
    pub lang: Lang,
    pub logo_color: String,
    pub service: Service,
    pub settings: Option<&'a Settings>,
    pub state: Option<&'a State>,
    pub providers: Option<&'a Result<Vec<ProviderInfo>, RegistryError>>,
    pub update_run: &'a UpdateRun,
    pub update_check: &'a CheckRun,
}

struct Pages {
    lang: Lang,
    logo_color: String,
    service: ServicePage,
    general: GeneralPage,
    accounts: AccountsPage,
    notifications: NotificationsPage,
    about: AboutPage,
}

pub struct SettingsWindow {
    window: adw::PreferencesWindow,
    act: Act,
    pages: RefCell<Option<Pages>>,
    ready: Cell<Option<bool>>,
    providers: RefCell<Option<Result<Vec<ProviderInfo>, RegistryError>>>,
    dialog: RefCell<Option<Rc<AddAccountDialog>>>,
}

impl SettingsWindow {
    #[must_use]
    pub fn new(application: Option<&adw::Application>, act: Act) -> Rc<Self> {
        let window = adw::PreferencesWindow::builder()
            .default_width(DEFAULT_WIDTH)
            .default_height(DEFAULT_HEIGHT)
            .search_enabled(false)
            .hide_on_close(true)
            .build();
        if let Some(application) = application {
            window.set_application(Some(application));
        }
        Rc::new(Self {
            window,
            act,
            pages: RefCell::default(),
            ready: Cell::default(),
            providers: RefCell::default(),
            dialog: RefCell::default(),
        })
    }

    pub fn present(&self) {
        self.window.present();
    }

    #[must_use]
    pub fn window(&self) -> &adw::PreferencesWindow {
        &self.window
    }

    pub fn update(self: &Rc<Self>, snapshot: &Snapshot) {
        *self.providers.borrow_mut() = snapshot.providers.cloned();
        let stale = self.pages.borrow().as_ref().is_none_or(|pages| {
            pages.lang != snapshot.lang || pages.logo_color != snapshot.logo_color
        });
        if stale {
            self.rebuild(snapshot);
        }
        let ready = snapshot.service == Service::Running
            && snapshot.settings.is_some()
            && snapshot.state.is_some();
        self.show_pages(ready);
        if let Some(pages) = self.pages.borrow().as_ref() {
            pages.service.update(snapshot.lang, &snapshot.service);
            pages.general.update(snapshot);
            pages.accounts.update(snapshot);
            pages.notifications.update(snapshot);
            pages.about.update(snapshot);
        }
    }

    fn rebuild(self: &Rc<Self>, snapshot: &Snapshot) {
        let visible = self.window.visible_page_name();
        self.show_nothing();
        let (lang, act) = (snapshot.lang, &self.act);
        let weak = Rc::downgrade(self);
        let on_add: Rc<dyn Fn()> = Rc::new(move || {
            if let Some(window) = weak.upgrade() {
                window.open_add_dialog();
            }
        });
        *self.pages.borrow_mut() = Some(Pages {
            lang,
            logo_color: snapshot.logo_color.clone(),
            service: ServicePage::new(lang, act, &snapshot.logo_color),
            general: GeneralPage::new(lang, act),
            accounts: AccountsPage::new(lang, act, on_add),
            notifications: NotificationsPage::new(lang, act),
            about: AboutPage::new(lang, act),
        });
        self.window
            .set_title(Some(&format!("Headroom — {}", lang.tr("Settings"))));
        self.ready.set(None);
        if let Some(name) = visible {
            self.window.set_visible_page_name(&name);
        }
    }

    fn show_nothing(&self) {
        if let Some(pages) = self.pages.borrow().as_ref() {
            for page in pages.all() {
                if page.parent().is_some() {
                    self.window.remove(page);
                }
            }
        }
    }

    fn show_pages(&self, ready: bool) {
        if self.ready.get() == Some(ready) {
            return;
        }
        self.show_nothing();
        if let Some(pages) = self.pages.borrow().as_ref() {
            let shown: Vec<&adw::PreferencesPage> = if ready {
                vec![
                    &pages.general.page,
                    &pages.accounts.page,
                    &pages.notifications.page,
                    &pages.about.page,
                ]
            } else {
                vec![&pages.service.page, &pages.about.page]
            };
            for page in shown {
                self.window.add(page);
            }
        }
        self.ready.set(Some(ready));
    }

    pub fn open_add_dialog(self: &Rc<Self>) {
        let Some((lang, logo_color)) = self
            .pages
            .borrow()
            .as_ref()
            .map(|pages| (pages.lang, pages.logo_color.clone()))
        else {
            return;
        };
        let weak = Rc::downgrade(self);
        let ctx = DialogCtx {
            lang,
            logo_color,
            act: Rc::clone(&self.act),
            close: Rc::new(move || {
                if let Some(dialog) = weak
                    .upgrade()
                    .and_then(|window| window.dialog.borrow().clone())
                {
                    dialog.dialog.close();
                }
            }),
        };
        let dialog = AddAccountDialog::new(ctx, self.providers.borrow().as_ref());
        dialog.dialog.present(Some(&self.window));
        *self.dialog.borrow_mut() = Some(dialog);
    }

    pub fn focus_account(&self, account_id: &str) {
        let pages = self.pages.borrow();
        let Some(pages) = pages.as_ref() else {
            return;
        };
        if pages.accounts.page.parent().is_some() {
            self.window.set_visible_page(&pages.accounts.page);
            pages.accounts.focus(account_id);
        }
    }

    pub fn restored(&self, result: &Result<(), String>) {
        if let Some(dialog) = self.dialog.borrow().as_ref() {
            dialog.restored(result);
        }
    }

    pub fn toast(&self, text: &str) {
        let toast = adw::Toast::builder()
            .title(text)
            .timeout(TOAST_SECONDS)
            .use_markup(false)
            .build();
        self.window.add_toast(toast);
    }

    #[must_use]
    pub fn select_page(&self, index: usize) -> bool {
        let pages = self.pages.borrow();
        let Some(page) = pages
            .as_ref()
            .and_then(|pages| pages.all().get(index).copied().cloned())
        else {
            return false;
        };
        if page.parent().is_none() {
            return false;
        }
        self.window.set_visible_page(&page);
        true
    }
}

impl Pages {
    fn all(&self) -> [&adw::PreferencesPage; 5] {
        [
            &self.general.page,
            &self.accounts.page,
            &self.notifications.page,
            &self.about.page,
            &self.service.page,
        ]
    }
}
