mod handlers;
mod render;
mod runtime;
mod subprocess;

use std::cell::RefCell;
use std::collections::BTreeSet;
use std::rc::Rc;

use adw::prelude::*;
use gtk::{gdk, gio, glib};

use crate::events::{Command, TrayUpdate};
use crate::icon::{Pixmap, SIZES, pixmap_from_rgba};
use crate::palette::Palettes;
use crate::ui::context::{Tick, UiState};
use crate::ui::style::Styles;
use crate::ui::svg_texture_at;
use crate::ui::window::PopupWindow;
use crate::view::View;

type Running = (Rc<App>, gio::ApplicationHoldGuard);

pub const APP_ID: &str = "io.github.daniarjabagin.HeadroomTray";
pub const TOGGLE_FLAG: &str = "--toggle";
const MARK_COLOR: &str = "#bebebe";

#[derive(Default)]
struct Model {
    view: View,
    reduced_motion_setting: bool,
    sign_in: BTreeSet<String>,
    ui: UiState,
    refresh_pressed: bool,
    refresh_failed: bool,
    ticks: Vec<Tick>,
    last_tray: Option<TrayUpdate>,
}

pub struct App {
    application: adw::Application,
    model: RefCell<Model>,
    window: RefCell<PopupWindow>,
    styles: RefCell<Styles>,
    palettes: Palettes,
    commands: tokio::sync::mpsc::UnboundedSender<Command>,
    tray: tokio::sync::mpsc::UnboundedSender<TrayUpdate>,
    ticker: RefCell<Option<glib::SourceId>>,
}

fn mark_pixmaps() -> Vec<Pixmap> {
    SIZES
        .iter()
        .filter_map(|size| {
            let texture = svg_texture_at(crate::assets::MARK, MARK_COLOR, *size)?;
            Some(pixmap_from_rgba(*size, &texture_pixels(&texture)?))
        })
        .collect()
}

fn texture_pixels(texture: &gdk::Texture) -> Option<Vec<u8>> {
    let mut downloader = gdk::TextureDownloader::new(texture);
    downloader.set_format(gdk::MemoryFormat::R8g8b8a8);
    let (bytes, stride) = downloader.download_bytes();
    let width = usize::try_from(texture.width()).ok()? * 4;
    Some(
        bytes
            .chunks(stride)
            .flat_map(|row| row[..width].to_vec())
            .collect(),
    )
}

impl App {
    fn startup(application: &adw::Application) -> anyhow::Result<Rc<Self>> {
        let display = gdk::Display::default().ok_or_else(|| anyhow::anyhow!("no display"))?;
        let palettes = Palettes::load()?;
        let first = render::initial_tray_update();
        let channels = runtime::start(mark_pixmaps(), first)?;
        let app = Rc::new(Self {
            application: application.clone(),
            model: RefCell::new(Model::default()),
            window: RefCell::new(PopupWindow::new(application)),
            styles: RefCell::new(Styles::install(&display)),
            palettes,
            commands: channels.commands,
            tray: channels.tray,
            ticker: RefCell::new(None),
        });
        app.listen(channels.events);
        app.watch_theme();
        app.watch_hidden();
        render::start_tray_clock(&app);
        Ok(app)
    }

    fn listen(self: &Rc<Self>, events: async_channel::Receiver<crate::events::Event>) {
        let weak = Rc::downgrade(self);
        glib::spawn_future_local(async move {
            while let Ok(event) = events.recv().await {
                let Some(app) = weak.upgrade() else { return };
                app.handle(event);
            }
        });
    }

    fn watch_theme(self: &Rc<Self>) {
        let weak = Rc::downgrade(self);
        adw::StyleManager::default().connect_dark_notify(move |_| {
            if let Some(app) = weak.upgrade() {
                app.refresh_look();
            }
        });
        let weak = Rc::downgrade(self);
        if let Some(settings) = gtk::Settings::default() {
            settings.connect_gtk_theme_name_notify(move |_| {
                if let Some(app) = weak.upgrade() {
                    app.refresh_look();
                }
            });
        }
    }

    fn watch_hidden(self: &Rc<Self>) {
        let weak = Rc::downgrade(self);
        self.window.borrow().connect_hidden(move || {
            if let Some(app) = weak.upgrade() {
                app.stop_ticker();
            }
        });
    }

    fn send(&self, command: Command) {
        if self.commands.send(command).is_err() {
            tracing::warn!("the D-Bus worker stopped");
        }
    }

    fn toggle(self: &Rc<Self>, click: Option<(i32, i32)>) {
        if self.window.borrow().is_visible() {
            self.window.borrow().hide();
        } else {
            self.show(click);
        }
    }

    fn show(self: &Rc<Self>, click: Option<(i32, i32)>) {
        tracing::debug!(?click, "opening the popup");
        self.window.borrow_mut().set_click(click);
        self.send(Command::Refresh(String::new()));
        self.render(true);
        self.window.borrow().show();
        self.start_ticker();
    }
}

#[must_use]
pub fn run(args: &[String]) -> glib::ExitCode {
    let application = adw::Application::builder()
        .application_id(APP_ID)
        .flags(gio::ApplicationFlags::HANDLES_COMMAND_LINE)
        .build();
    let state: Rc<RefCell<Option<Running>>> = Rc::default();
    let started = Rc::clone(&state);
    application.connect_startup(move |application| match App::startup(application) {
        Ok(app) => *started.borrow_mut() = Some((app, application.hold())),
        Err(error) => tracing::error!(%error, "headroom-tray could not start"),
    });
    application.connect_command_line(move |_, line| {
        let toggle = line
            .arguments()
            .iter()
            .any(|arg| arg.to_str() == Some(TOGGLE_FLAG));
        if let Some((app, _)) = state.borrow().as_ref() {
            if toggle {
                app.toggle(None);
            }
            return glib::ExitCode::SUCCESS;
        }
        glib::ExitCode::FAILURE
    });
    application.run_with_args(args)
}
