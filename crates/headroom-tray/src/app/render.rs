use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use gtk::glib;
use jiff::Timestamp;
use jiff::tz::TimeZone;

use super::App;
use crate::dates::Locale;
use crate::events::{MenuLabels, TrayUpdate};
use crate::i18n::{Lang, system_locale};
use crate::icon::ring_pixmaps;
use crate::palette::{Palette, Scheme};
use crate::payload::Display;
use crate::ui::context::{Action, Ctx, RefreshMode};
use crate::ui::popup::Frame;
use crate::ui::style::resolve_theme;
use crate::ui::{build, reduced_motion};
use crate::view::tray_look;

const TICK_SECONDS: u32 = 1;
const TRAY_CLOCK_SECONDS: u32 = 30;

fn locale(display: &Display) -> Locale {
    let lang = Lang::resolve(display.language, system_locale().as_deref());
    Locale::new(lang, TimeZone::system())
}

fn menu_labels(lang: Lang) -> MenuLabels {
    MenuLabels {
        open: lang.tr("Open Headroom").to_owned(),
        refresh: lang.tr("Refresh now").to_owned(),
        quit: lang.tr("Quit").to_owned(),
    }
}

pub(super) fn initial_tray_update() -> TrayUpdate {
    let locale = locale(&Display::default());
    let look = tray_look(&crate::view::View::Loading, &locale, Timestamp::now());
    TrayUpdate {
        ring: None,
        tooltip: look.tooltip,
        labels: menu_labels(locale.lang),
    }
}

pub(super) fn start_tray_clock(app: &Rc<App>) {
    let weak = Rc::downgrade(app);
    glib::timeout_add_seconds_local(TRAY_CLOCK_SECONDS, move || {
        let Some(app) = weak.upgrade() else {
            return glib::ControlFlow::Break;
        };
        app.sync_tray();
        glib::ControlFlow::Continue
    });
}

impl App {
    fn display(&self) -> Display {
        self.model
            .borrow()
            .view
            .state()
            .map(|state| state.display.clone())
            .unwrap_or_default()
    }

    fn palette_for(&self, scheme: Scheme) -> &Palette {
        match scheme {
            Scheme::Light => &self.palettes.light,
            Scheme::Dark => &self.palettes.dark,
        }
    }

    fn palette(&self, display: &Display) -> Palette {
        let palette = self.palette_for(resolve_theme(display.theme));
        if let Err(error) = self.styles.borrow_mut().apply(palette) {
            tracing::error!(%error, "the stylesheet could not be generated");
        }
        palette.clone()
    }

    fn refresh_mode(&self) -> RefreshMode {
        let model = self.model.borrow();
        let busy = model
            .view
            .state()
            .is_some_and(crate::payload::State::is_refreshing);
        if model.refresh_failed {
            RefreshMode::Failed
        } else if model.refresh_pressed || busy {
            RefreshMode::Busy
        } else {
            RefreshMode::Idle
        }
    }

    fn context(self: &Rc<Self>, display: Display) -> Ctx {
        let weak = Rc::downgrade(self);
        let act: Rc<dyn Fn(Action)> = Rc::new(move |action| {
            if let Some(app) = weak.upgrade() {
                app.act(action);
            }
        });
        let refresh = self.refresh_mode();
        let model = self.model.borrow();
        let mut ui = model.ui.clone();
        ui.refresh = refresh;
        Ctx {
            locale: locale(&display),
            palette: self.palette(&display),
            motion: !reduced_motion(model.reduced_motion_setting),
            offline: model.view.state().is_some_and(|state| state.offline),
            sign_in: model.sign_in.clone(),
            ui,
            version: format!("Headroom {}", env!("CARGO_PKG_VERSION")),
            act,
            ticks: RefCell::default(),
            display,
        }
    }

    pub(super) fn render(self: &Rc<Self>, entrance: bool) {
        let display = self.display();
        let ctx = self.context(display);
        let visible = self.window.borrow().is_visible();
        if !visible && !entrance {
            return;
        }
        let frame = Frame {
            now: Timestamp::now(),
            max_height: self.window.borrow().max_height(),
            animate: entrance && ctx.motion,
        };
        let view = self.model.borrow().view.clone();
        let (root, ticks) = build(ctx, &view, &frame);
        self.model.borrow_mut().ticks = ticks;
        self.window.borrow().set_content(&root);
    }

    pub(super) fn refresh_look(self: &Rc<Self>) {
        self.render(false);
        self.sync_tray();
    }

    pub(super) fn sync_tray(&self) {
        let display = self.display();
        let locale = locale(&display);
        let look = tray_look(&self.model.borrow().view, &locale, Timestamp::now());
        let palette = self.palette_for(resolve_theme(display.theme));
        let ring = look.ring.and_then(|key| {
            let color = palette.tone(key.tone).ok()?;
            Some(ring_pixmaps(key, color))
        });
        let update = TrayUpdate {
            ring,
            tooltip: look.tooltip,
            labels: menu_labels(locale.lang),
        };
        let mut model = self.model.borrow_mut();
        if model.last_tray.as_ref() == Some(&update) {
            return;
        }
        model.last_tray = Some(update.clone());
        if self.tray.send(update).is_err() {
            tracing::warn!("the tray worker stopped");
        }
    }

    pub(super) fn start_ticker(self: &Rc<Self>) {
        self.stop_ticker();
        let weak = Rc::downgrade(self);
        let source =
            glib::timeout_add_local(Duration::from_secs(u64::from(TICK_SECONDS)), move || {
                let Some(app) = weak.upgrade() else {
                    return glib::ControlFlow::Break;
                };
                let now = Timestamp::now();
                for tick in &app.model.borrow().ticks {
                    tick(now);
                }
                glib::ControlFlow::Continue
            });
        *self.ticker.borrow_mut() = Some(source);
    }

    pub(super) fn stop_ticker(&self) {
        if let Some(source) = self.ticker.borrow_mut().take() {
            source.remove();
        }
    }
}
