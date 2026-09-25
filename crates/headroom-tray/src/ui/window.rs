use std::rc::Rc;

use gtk::prelude::*;
use gtk::{gdk, gio};

use crate::placement::{POPUP_WIDTH, Rect, max_popup_height, monitor_for, usable_click};
use crate::ui::x11::{X11Placer, place_after_map};

enum Backend {
    #[cfg(feature = "layer-shell")]
    Layer,
    X11(Rc<X11Placer>),
    Plain,
}

pub struct PopupWindow {
    window: gtk::Window,
    backend: Backend,
    click: Option<(i32, i32)>,
}

fn rect(monitor: &gdk::Monitor) -> Rect {
    let geometry = monitor.geometry();
    Rect {
        x: geometry.x(),
        y: geometry.y(),
        width: geometry.width(),
        height: geometry.height(),
    }
}

fn monitors(window: &gtk::Window) -> Vec<gdk::Monitor> {
    let list = gtk::prelude::WidgetExt::display(window).monitors();
    (0..list.n_items())
        .filter_map(|index| list.item(index)?.downcast::<gdk::Monitor>().ok())
        .collect()
}

fn pick_monitor(window: &gtk::Window, click: Option<(i32, i32)>) -> Option<gdk::Monitor> {
    let all = monitors(window);
    let rects: Vec<Rect> = all.iter().map(rect).collect();
    let index = monitor_for(&rects, click).unwrap_or(0);
    all.get(index).cloned()
}

fn backend(window: &gtk::Window) -> Backend {
    #[cfg(feature = "layer-shell")]
    if crate::ui::layer::supported() {
        crate::ui::layer::init(window);
        return Backend::Layer;
    }
    if gtk::prelude::WidgetExt::display(window).is::<gdk4_x11::X11Display>() {
        match X11Placer::connect().and_then(|placer| placer.prepare(window).map(|()| placer)) {
            Ok(placer) => return Backend::X11(Rc::new(placer)),
            Err(error) => tracing::warn!(%error, "falling back to a centered popup"),
        }
    }
    Backend::Plain
}

fn close_on_escape(window: &gtk::Window) {
    let keys = gtk::EventControllerKey::new();
    let target = window.clone();
    keys.connect_key_pressed(move |_, key, _, _| {
        if key == gdk::Key::Escape {
            target.set_visible(false);
            return gtk::glib::Propagation::Stop;
        }
        gtk::glib::Propagation::Proceed
    });
    window.add_controller(keys);
}

impl PopupWindow {
    #[must_use]
    pub fn new(application: &impl IsA<gtk::Application>) -> Self {
        let window = gtk::Window::builder()
            .application(application)
            .title("Headroom")
            .decorated(false)
            .resizable(false)
            .default_width(POPUP_WIDTH)
            .hide_on_close(true)
            .css_classes(["headroom-window"])
            .build();
        close_on_escape(&window);
        window.connect_is_active_notify(|window| {
            if !window.is_active() {
                window.set_visible(false);
            }
        });
        let backend = backend(&window);
        Self {
            window,
            backend,
            click: None,
        }
    }

    #[must_use]
    pub fn is_visible(&self) -> bool {
        self.window.is_visible()
    }

    pub fn hide(&self) {
        self.window.set_visible(false);
    }

    pub fn connect_hidden(&self, on_hidden: impl Fn() + 'static) {
        self.window.connect_visible_notify(move |window| {
            if !window.is_visible() {
                on_hidden();
            }
        });
    }

    #[must_use]
    pub fn max_height(&self) -> i32 {
        pick_monitor(&self.window, self.click).map_or(600, |m| max_popup_height(rect(&m).height))
    }

    pub fn set_click(&mut self, click: Option<(i32, i32)>) {
        self.click = usable_click(click);
    }

    pub fn set_content(&self, content: &impl IsA<gtk::Widget>) {
        self.window.set_child(Some(content));
    }

    pub fn show(&self) {
        let monitor = pick_monitor(&self.window, self.click);
        match &self.backend {
            #[cfg(feature = "layer-shell")]
            Backend::Layer => {
                if let Some(monitor) = &monitor {
                    crate::ui::layer::place(&self.window, monitor, rect(monitor), self.click);
                }
                self.window.present();
            }
            Backend::X11(placer) => {
                self.window.present();
                if let (Some(monitor), Some(click)) = (monitor, self.click) {
                    place_after_map(Rc::clone(placer), &self.window, rect(&monitor), click);
                }
            }
            Backend::Plain => self.window.present(),
        }
    }

    #[must_use]
    pub fn widget(&self) -> gtk::Widget {
        self.window.clone().upcast()
    }

    #[must_use]
    pub fn clipboard(&self) -> gdk::Clipboard {
        gtk::prelude::WidgetExt::display(&self.window).clipboard()
    }

    pub fn open_uri(&self, uri: &str) {
        let launcher = gtk::UriLauncher::new(uri);
        launcher.launch(Some(&self.window), gio::Cancellable::NONE, |result| {
            if let Err(error) = result {
                tracing::warn!(%error, "could not open a link");
            }
        });
    }
}
