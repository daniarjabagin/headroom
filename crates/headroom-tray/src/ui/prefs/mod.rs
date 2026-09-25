mod about;
mod account_row;
mod accounts;
pub mod add_account;
mod advanced;
mod cards_group;
mod detect_page;
pub mod diagnostics;
mod flow_page;
mod general;
mod key_page;
pub mod keyboard;
mod limits;
mod login_page;
mod notifications;
mod panel_group;
mod picks;
mod privacy_group;
mod quiet;
mod rows;
mod service;
pub mod shortcut;
mod spend_group;
pub mod target;
mod thresholds;

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use adw::prelude::*;

use crate::i18n::Lang;
use crate::payload::{Account, State};
use crate::preferences::capability::Capabilities;
use crate::preferences::change::Change;
use crate::preferences::model::Settings;
use crate::preferences::registry::{ProviderInfo, RegistryError};
use crate::update::UpdateRun;
use crate::update_check::CheckRun;
use about::AboutPage;
use accounts::AccountsPage;
use add_account::{AddAccountDialog, DialogCtx};
use advanced::{AdvancedPage, Toast};
use diagnostics::Diagnostics;
use general::GeneralPage;
use notifications::NotificationsPage;
use service::ServicePage;
use target::login_method;

pub use account_row::provider_image;
pub use rows::pill_button;

const DEFAULT_WIDTH: i32 = 900;
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
    RestartService,
    InstallUpdate,
    OpenUrl(String),
    CheckForUpdates,
    CopyDiagnostics,
    ResetSettings,
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
    advanced: AdvancedPage,
    about: AboutPage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Shown {
    ready: bool,
    advanced: bool,
}

pub struct SettingsWindow {
    window: adw::PreferencesWindow,
    act: Act,
    pages: RefCell<Option<Pages>>,
    shown: Cell<Option<Shown>>,
    providers: RefCell<Option<Result<Vec<ProviderInfo>, RegistryError>>>,
    dialog: RefCell<Option<Rc<AddAccountDialog>>>,
    log_file: RefCell<Option<String>>,
    copy_pending: Cell<bool>,
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
            shown: Cell::default(),
            providers: RefCell::default(),
            dialog: RefCell::default(),
            log_file: RefCell::default(),
            copy_pending: Cell::default(),
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
        let advanced = ready && Capabilities::of(snapshot.state).release_0_6;
        self.show_pages(Shown { ready, advanced });
        if let Some(pages) = self.pages.borrow().as_ref() {
            pages.service.update(snapshot.lang, &snapshot.service);
            pages.general.update(snapshot);
            pages.accounts.update(snapshot);
            pages.notifications.update(snapshot);
            pages.advanced.update(snapshot);
            pages.about.update(snapshot);
        }
    }

    fn toaster(self: &Rc<Self>) -> Toast {
        let weak = Rc::downgrade(self);
        Rc::new(move |text: &str| {
            if let Some(window) = weak.upgrade() {
                window.toast(text);
            }
        })
    }

    fn rebuild(self: &Rc<Self>, snapshot: &Snapshot) {
        let visible = self.window.visible_page_name();
        self.show_nothing();
        let (lang, act, color) = (snapshot.lang, &self.act, snapshot.logo_color.as_str());
        let weak = Rc::downgrade(self);
        let on_add: Rc<dyn Fn()> = Rc::new(move || {
            if let Some(window) = weak.upgrade() {
                window.open_add_dialog(None);
            }
        });
        let advanced = AdvancedPage::new(lang, act, &self.toaster());
        advanced.set_log_file(self.log_file.borrow().as_deref());
        *self.pages.borrow_mut() = Some(Pages {
            lang,
            logo_color: snapshot.logo_color.clone(),
            service: ServicePage::new(lang, act, color),
            general: GeneralPage::new(lang, act, color),
            accounts: AccountsPage::new(lang, act, on_add),
            notifications: NotificationsPage::new(lang, act, color),
            advanced,
            about: AboutPage::new(lang, act),
        });
        self.window
            .set_title(Some(&format!("Headroom — {}", lang.tr("Settings"))));
        self.shown.set(None);
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

    fn show_pages(&self, shown: Shown) {
        if self.shown.get() == Some(shown) {
            return;
        }
        self.show_nothing();
        if let Some(pages) = self.pages.borrow().as_ref() {
            let mut visible: Vec<&adw::PreferencesPage> = if shown.ready {
                vec![
                    &pages.general.page,
                    &pages.accounts.page,
                    &pages.notifications.page,
                ]
            } else {
                vec![&pages.service.page]
            };
            if shown.advanced {
                visible.push(&pages.advanced.page);
            }
            visible.push(&pages.about.page);
            for page in visible {
                self.window.add(page);
            }
        }
        self.shown.set(Some(shown));
    }

    fn dialog_ctx(self: &Rc<Self>) -> Option<DialogCtx> {
        let (lang, logo_color) = self
            .pages
            .borrow()
            .as_ref()
            .map(|pages| (pages.lang, pages.logo_color.clone()))?;
        let weak = Rc::downgrade(self);
        Some(DialogCtx {
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
        })
    }

    fn present_dialog(&self, dialog: Rc<AddAccountDialog>) {
        dialog.dialog.present(Some(&self.window));
        *self.dialog.borrow_mut() = Some(dialog);
    }

    pub fn open_add_dialog(self: &Rc<Self>, provider_id: Option<&str>) {
        if let Some(ctx) = self.dialog_ctx() {
            let dialog = AddAccountDialog::new(ctx, self.providers.borrow().as_ref(), provider_id);
            self.present_dialog(dialog);
        }
    }

    #[must_use]
    pub fn open_login_dialog(self: &Rc<Self>, provider_id: &str, account_id: &str) -> bool {
        let provider = self
            .providers
            .borrow()
            .as_ref()
            .and_then(|providers| providers.as_ref().ok())
            .and_then(|list| list.iter().find(|provider| provider.id == provider_id))
            .cloned();
        let Some((provider, ctx)) = provider.zip(self.dialog_ctx()) else {
            return false;
        };
        let Some(method) = login_method(&provider) else {
            return false;
        };
        let dialog = AddAccountDialog::relogin(ctx, &provider, method, account_id);
        self.present_dialog(dialog);
        true
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

    pub fn request_diagnostics_copy(&self) {
        self.copy_pending.set(true);
    }

    pub fn diagnostics(&self, lang: Lang, result: Result<Diagnostics, String>) {
        let copy = self.copy_pending.replace(false);
        match result {
            Ok(diagnostics) => {
                self.log_file.replace(diagnostics.log_file.clone());
                if let Some(pages) = self.pages.borrow().as_ref() {
                    pages.advanced.set_log_file(diagnostics.log_file.as_deref());
                }
                if copy {
                    self.window.clipboard().set_text(&diagnostics.text);
                    self.toast(lang.tr("Diagnostics copied"));
                }
            }
            Err(message) if copy => self.toast(&message),
            Err(message) => tracing::warn!(%message, "diagnostics are unavailable"),
        }
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
    fn all(&self) -> [&adw::PreferencesPage; 6] {
        [
            &self.general.page,
            &self.accounts.page,
            &self.notifications.page,
            &self.advanced.page,
            &self.about.page,
            &self.service.page,
        ]
    }
}
