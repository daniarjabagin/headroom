mod done;
pub mod found;
mod welcome;

use std::cell::RefCell;
use std::rc::Rc;

use adw::prelude::*;

use crate::assets::MARK;
use crate::i18n::Lang;
use crate::payload::State;
use crate::preferences::change::Change;
use crate::preferences::model::Settings;
use crate::preferences::registry::{ProviderInfo, RegistryError};
use crate::ui::prefs::add_account::{AddAccountDialog, DialogCtx};
use crate::ui::prefs::{Act, PrefsAction};
use crate::ui::widgets::svg_texture;
use done::Done;
use welcome::Welcome;

const WIDTH: i32 = 560;
const HEIGHT: i32 = 760;
const CONTENT_WIDTH: i32 = 480;
const MARGIN: i32 = 24;
const MARK_SIZE: i32 = 64;
const TILE_SIZE: i32 = 96;
const WELCOME: &str = "welcome";
const DONE: &str = "done";

type Providers = Result<Vec<ProviderInfo>, RegistryError>;

#[derive(Clone)]
struct Ctx {
    lang: Lang,
    logo_color: String,
    act: Act,
    on_add: Rc<dyn Fn(Option<String>)>,
    on_start: Rc<dyn Fn()>,
    on_later: Rc<dyn Fn()>,
    on_done: Rc<dyn Fn()>,
}

pub struct OnboardingWindow {
    window: adw::Window,
    stack: gtk::Stack,
    welcome: Welcome,
    done: Done,
    ctx: Ctx,
    providers: RefCell<Option<Providers>>,
    dialog: RefCell<Option<Rc<AddAccountDialog>>>,
}

pub struct OnboardingSetup {
    pub lang: Lang,
    pub logo_color: String,
    pub act: Act,
    pub shortcut_supported: bool,
    pub on_closed: Rc<dyn Fn()>,
}

fn page_column() -> gtk::Box {
    let column = gtk::Box::new(gtk::Orientation::Vertical, 18);
    column.set_margin_start(MARGIN);
    column.set_margin_end(MARGIN);
    column.set_margin_bottom(MARGIN);
    column.set_width_request(CONTENT_WIDTH);
    column.set_margin_top(MARGIN / 2);
    column.set_halign(gtk::Align::Center);
    column.set_valign(gtk::Align::Center);
    column
}

fn pinned_page(content: &gtk::Box, actions: &[gtk::Widget]) -> gtk::Box {
    let scroller = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vexpand(true)
        .child(content)
        .build();
    let footer = gtk::Box::new(gtk::Orientation::Vertical, 12);
    footer.set_margin_top(MARGIN / 2);
    footer.set_margin_bottom(MARGIN);
    footer.set_margin_start(MARGIN);
    footer.set_margin_end(MARGIN);
    for action in actions {
        footer.append(action);
    }
    let page = gtk::Box::new(gtk::Orientation::Vertical, 0);
    page.append(&scroller);
    page.append(&footer);
    page
}

fn mark_tile(ctx: &Ctx) -> gtk::Box {
    let tile = gtk::Box::new(gtk::Orientation::Vertical, 0);
    tile.add_css_class("card");
    tile.set_size_request(TILE_SIZE, TILE_SIZE);
    tile.set_halign(gtk::Align::Center);
    tile.set_valign(gtk::Align::Start);
    tile.set_vexpand(false);
    let image = gtk::Image::new();
    image.set_pixel_size(MARK_SIZE);
    image.set_valign(gtk::Align::Center);
    image.set_vexpand(true);
    if let Some(mark) = svg_texture(MARK, &ctx.logo_color, MARK_SIZE) {
        image.set_paintable(Some(&mark));
    }
    tile.append(&image);
    tile
}

fn title_block(ctx: &Ctx, title: &str, subtitle: &str) -> gtk::Box {
    let block = gtk::Box::new(gtk::Orientation::Vertical, 8);
    let heading = gtk::Label::new(Some(title));
    heading.add_css_class("title-1");
    heading.set_wrap(true);
    heading.set_justify(gtk::Justification::Center);
    let text = gtk::Label::new(Some(subtitle));
    text.add_css_class("dim-label");
    text.set_wrap(true);
    text.set_justify(gtk::Justification::Center);
    block.append(&mark_tile(ctx));
    block.append(&heading);
    block.append(&text);
    block
}

impl OnboardingWindow {
    #[must_use]
    pub fn new(application: Option<&adw::Application>, setup: &OnboardingSetup) -> Rc<Self> {
        let window = adw::Window::builder()
            .default_width(WIDTH)
            .default_height(HEIGHT)
            .title(setup.lang.tr("Welcome to Headroom"))
            .build();
        if let Some(application) = application {
            window.set_application(Some(application));
        }
        let closed = Rc::clone(&setup.on_closed);
        window.connect_close_request(move |_| {
            closed();
            gtk::glib::Propagation::Proceed
        });
        Rc::new_cyclic(|weak: &std::rc::Weak<Self>| {
            let ctx = Self::context(weak, setup);
            let stack = gtk::Stack::builder()
                .transition_type(gtk::StackTransitionType::Crossfade)
                .vhomogeneous(false)
                .build();
            let welcome = Welcome::new(&ctx);
            let done = Done::new(&ctx, setup.shortcut_supported);
            stack.add_named(&welcome.widget, Some(WELCOME));
            stack.add_named(&done.widget, Some(DONE));
            let header = adw::HeaderBar::builder().show_title(false).build();
            let toolbar = adw::ToolbarView::new();
            toolbar.add_top_bar(&header);
            toolbar.set_content(Some(&stack));
            window.set_content(Some(&toolbar));
            Self {
                window,
                stack,
                welcome,
                done,
                ctx,
                providers: RefCell::default(),
                dialog: RefCell::default(),
            }
        })
    }

    fn context(weak: &std::rc::Weak<Self>, setup: &OnboardingSetup) -> Ctx {
        let (add, start, later, done) = (weak.clone(), weak.clone(), weak.clone(), weak.clone());
        let act = Rc::clone(&setup.act);
        Ctx {
            lang: setup.lang,
            logo_color: setup.logo_color.clone(),
            act: Rc::clone(&setup.act),
            on_add: Rc::new(move |provider: Option<String>| {
                if let Some(window) = add.upgrade() {
                    window.open_add_dialog(provider.as_deref());
                }
            }),
            on_start: Rc::new(move || {
                act(PrefsAction::Change(Change::OnboardingCompleted(true)));
                if let Some(window) = start.upgrade() {
                    window.stack.set_visible_child_name(DONE);
                }
            }),
            on_later: Rc::new(move || {
                if let Some(window) = later.upgrade() {
                    window.window.close();
                }
            }),
            on_done: Rc::new(move || {
                if let Some(window) = done.upgrade() {
                    window.window.close();
                }
            }),
        }
    }

    pub fn present(&self) {
        self.window.present();
    }

    #[must_use]
    pub fn window(&self) -> &adw::Window {
        &self.window
    }

    pub fn show_done(&self) {
        self.stack.set_visible_child_name(DONE);
    }

    pub fn update(&self, state: &State, settings: &Settings, providers: Option<&Providers>) {
        *self.providers.borrow_mut() = providers.cloned();
        let list = providers
            .and_then(|providers| providers.as_ref().ok())
            .map_or(&[][..], Vec::as_slice);
        self.welcome.update(state, list);
        self.done.update(&settings.shortcuts.open);
    }

    fn open_add_dialog(self: &Rc<Self>, provider: Option<&str>) {
        let weak = Rc::downgrade(self);
        let ctx = DialogCtx {
            lang: self.ctx.lang,
            logo_color: self.ctx.logo_color.clone(),
            act: Rc::clone(&self.ctx.act),
            close: Rc::new(move || {
                if let Some(dialog) = weak
                    .upgrade()
                    .and_then(|window| window.dialog.borrow().clone())
                {
                    dialog.dialog.close();
                }
            }),
        };
        let dialog = AddAccountDialog::new(ctx, self.providers.borrow().as_ref(), provider);
        dialog.dialog.present(Some(&self.window));
        *self.dialog.borrow_mut() = Some(dialog);
    }

    pub fn restored(&self, result: &Result<(), String>) {
        if let Some(dialog) = self.dialog.borrow().as_ref() {
            dialog.restored(result);
        }
    }
}
