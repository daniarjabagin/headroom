use std::cell::RefCell;
use std::rc::Rc;

use adw::prelude::*;

use super::Act;
use super::account_list::provider_image;
use super::detect_page::DetectPage;
use super::flow_page::{navigation_page, wrapping};
use super::key_page::KeyPage;
use super::login_page::{LoginPage, with};
use super::target::Target;
use crate::i18n::{Lang, fill};
use crate::preferences::registry::{
    AddMethod, ProviderInfo, RegistryError, method_summary, provider_summary,
};

const CONTENT_WIDTH: i32 = 460;
const CONTENT_HEIGHT: i32 = 600;
const LIST_MARGIN: i32 = 18;
const ROW_LOGO: i32 = 24;

#[derive(Clone)]
pub struct DialogCtx {
    pub lang: Lang,
    pub logo_color: String,
    pub act: Act,
    pub close: Rc<dyn Fn()>,
}

enum FlowPage {
    Login(Rc<LoginPage>),
    Key(Rc<KeyPage>),
    Detect(Rc<DetectPage>),
}

impl FlowPage {
    fn page(&self) -> &adw::NavigationPage {
        match self {
            FlowPage::Login(flow) => &flow.page,
            FlowPage::Key(flow) => &flow.page,
            FlowPage::Detect(flow) => &flow.page,
        }
    }

    fn cancel(&self) {
        match self {
            FlowPage::Login(flow) => flow.cancel(),
            FlowPage::Key(flow) => flow.cancel(),
            FlowPage::Detect(_) => {}
        }
    }
}

pub struct AddAccountDialog {
    pub dialog: adw::Dialog,
    navigation: adw::NavigationView,
    ctx: DialogCtx,
    flows: RefCell<Vec<FlowPage>>,
}

fn choice_row(
    title: &str,
    subtitle: &str,
    prefix: Option<gtk::Image>,
    on_activate: impl Fn() + 'static,
) -> adw::ActionRow {
    let row = adw::ActionRow::builder()
        .title(title)
        .subtitle(subtitle)
        .activatable(true)
        .use_markup(false)
        .build();
    if let Some(prefix) = prefix {
        row.add_prefix(&prefix);
    }
    row.add_suffix(&gtk::Image::from_icon_name("go-next-symbolic"));
    row.connect_activated(move |_| on_activate());
    row
}

fn list_content(heading: &str, rows: &[adw::ActionRow]) -> gtk::Box {
    let column = gtk::Box::new(gtk::Orientation::Vertical, 12);
    for margin in [
        gtk::Box::set_margin_top,
        gtk::Box::set_margin_bottom,
        gtk::Box::set_margin_start,
        gtk::Box::set_margin_end,
    ] {
        margin(&column, LIST_MARGIN);
    }
    let intro = wrapping(heading, &["dim-label"]);
    intro.set_xalign(0.0);
    intro.set_justify(gtk::Justification::Left);
    let list = gtk::ListBox::new();
    list.set_selection_mode(gtk::SelectionMode::None);
    list.add_css_class("boxed-list");
    for row in rows {
        list.append(row);
    }
    column.append(&intro);
    column.append(&list);
    column
}

fn unavailable_page(lang: Lang, error: Option<String>) -> adw::NavigationPage {
    let status = adw::StatusPage::builder()
        .icon_name("dialog-warning-symbolic")
        .title(lang.tr("No providers available"))
        .description(glib_escape(&error.unwrap_or_else(|| {
            lang.tr("The Headroom service did not list any providers.")
                .to_owned()
        })))
        .build();
    navigation_page(lang.tr("Add Account"), &status)
}

fn glib_escape(text: &str) -> String {
    gtk::glib::markup_escape_text(text).to_string()
}

impl AddAccountDialog {
    fn empty(ctx: DialogCtx) -> Rc<Self> {
        let dialog = adw::Dialog::builder()
            .content_width(CONTENT_WIDTH)
            .content_height(CONTENT_HEIGHT)
            .build();
        let navigation = adw::NavigationView::new();
        dialog.set_child(Some(&navigation));
        let this = Rc::new(Self {
            dialog,
            navigation,
            ctx,
            flows: RefCell::default(),
        });
        this.connect();
        this
    }

    #[must_use]
    pub fn relogin(
        ctx: DialogCtx,
        provider: &ProviderInfo,
        method: &AddMethod,
        account_id: &str,
    ) -> Rc<Self> {
        let this = Self::empty(ctx);
        this.start_flow(
            provider,
            method,
            &Target::Login {
                account_id: account_id.to_owned(),
            },
        );
        this
    }

    #[must_use]
    pub fn new(
        ctx: DialogCtx,
        providers: Option<&Result<Vec<ProviderInfo>, RegistryError>>,
        provider_id: Option<&str>,
    ) -> Rc<Self> {
        let this = Self::empty(ctx);
        let first = match providers {
            Some(Ok(list)) if !list.is_empty() => this.provider_page(list),
            Some(Err(error)) => unavailable_page(this.ctx.lang, Some(error.message(this.ctx.lang))),
            _ => unavailable_page(this.ctx.lang, None),
        };
        this.navigation.push(&first);
        let picked = providers
            .and_then(|providers| providers.as_ref().ok())
            .and_then(|list| {
                list.iter()
                    .find(|provider| Some(provider.id.as_str()) == provider_id)
            });
        if let Some(provider) = picked {
            this.pick_provider(provider);
        }
        this
    }

    fn connect(self: &Rc<Self>) {
        let weak = Rc::downgrade(self);
        self.navigation
            .connect_popped(move |_, page| with(&weak, |dialog| dialog.drop_flow(page)));
        let weak = Rc::downgrade(self);
        self.dialog
            .connect_closed(move |_| with(&weak, |dialog| dialog.cancel_all()));
    }

    fn provider_page(self: &Rc<Self>, providers: &[ProviderInfo]) -> adw::NavigationPage {
        let lang = self.ctx.lang;
        let rows: Vec<adw::ActionRow> = providers
            .iter()
            .map(|provider| {
                let (weak, picked) = (Rc::downgrade(self), provider.clone());
                let logo = provider_image(&provider.id, &self.ctx.logo_color, ROW_LOGO);
                choice_row(
                    &provider.display_name,
                    &provider_summary(lang, provider),
                    Some(logo),
                    move || {
                        with(&weak, |dialog| dialog.pick_provider(&picked));
                    },
                )
            })
            .collect();
        navigation_page(
            lang.tr("Add Account"),
            &list_content(lang.tr("Choose the service to track."), &rows),
        )
    }

    fn pick_provider(self: &Rc<Self>, provider: &ProviderInfo) {
        if let [method] = provider.methods.as_slice() {
            self.start_flow(provider, method, &Target::Add);
            return;
        }
        let lang = self.ctx.lang;
        let rows: Vec<adw::ActionRow> = provider
            .methods
            .iter()
            .map(|method| {
                let (weak, provider, method) =
                    (Rc::downgrade(self), provider.clone(), method.clone());
                choice_row(&method_summary(lang, &method), "", None, move || {
                    with(&weak, |dialog| {
                        dialog.start_flow(&provider, &method, &Target::Add);
                    });
                })
            })
            .collect();
        let heading = fill(
            lang.tr("How do you want to add your {provider} account?"),
            &[("provider", &provider.display_name)],
        );
        self.navigation.push(&navigation_page(
            &provider.display_name,
            &list_content(&heading, &rows),
        ));
    }

    fn start_flow(&self, provider: &ProviderInfo, method: &AddMethod, target: &Target) {
        let flow = match method {
            AddMethod::CliLogin { .. } => {
                FlowPage::Login(LoginPage::new(&self.ctx, provider, method, target))
            }
            AddMethod::ApiKey { .. } => {
                FlowPage::Key(KeyPage::new(&self.ctx, provider, method, target))
            }
            AddMethod::AutoDetect { .. } => {
                FlowPage::Detect(DetectPage::new(&self.ctx, provider, method))
            }
        };
        self.navigation.push(flow.page());
        self.flows.borrow_mut().push(flow);
    }

    fn drop_flow(&self, page: &adw::NavigationPage) {
        let mut flows = self.flows.borrow_mut();
        if let Some(index) = flows.iter().position(|flow| flow.page() == page) {
            flows.remove(index).cancel();
        }
    }

    fn cancel_all(&self) {
        for flow in self.flows.borrow_mut().drain(..) {
            flow.cancel();
        }
    }

    pub fn restored(&self, result: &Result<(), String>) {
        for flow in self.flows.borrow().iter() {
            if let FlowPage::Detect(page) = flow {
                page.restored(result);
            }
        }
    }
}
