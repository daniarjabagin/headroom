use gdk4_x11::X11Surface;
use gtk::glib;
use gtk::prelude::*;
use x11rb::connection::Connection;
use x11rb::protocol::xproto::{
    AtomEnum, ClientMessageEvent, ConfigureWindowAux, ConnectionExt, EventMask, PropMode,
};
use x11rb::rust_connection::RustConnection;
use x11rb::wrapper::ConnectionExt as _;

use crate::placement::{Rect, window_position};

const STATE_ADD: u32 = 1;
const SOURCE_APPLICATION: u32 = 1;

#[derive(Debug, thiserror::Error)]
pub enum X11Error {
    #[error("could not connect to the X server: {0}")]
    Connect(#[from] x11rb::errors::ConnectError),
    #[error("X11 request failed: {0}")]
    Connection(#[from] x11rb::errors::ConnectionError),
    #[error("X11 reply failed: {0}")]
    Reply(#[from] x11rb::errors::ReplyError),
    #[error("the popup has no X11 window")]
    NoWindow,
}

pub struct X11Placer {
    connection: RustConnection,
    root: u32,
}

fn window_id(window: &gtk::Window) -> Result<u32, X11Error> {
    let surface = window.surface().ok_or(X11Error::NoWindow)?;
    let x11 = surface
        .downcast::<X11Surface>()
        .map_err(|_| X11Error::NoWindow)?;
    u32::try_from(x11.xid()).map_err(|_| X11Error::NoWindow)
}

fn skip_taskbar(window: &gtk::Window) {
    if let Some(surface) = window
        .surface()
        .and_then(|s| s.downcast::<X11Surface>().ok())
    {
        surface.set_skip_taskbar_hint(true);
        surface.set_skip_pager_hint(true);
    }
}

impl X11Placer {
    pub fn connect() -> Result<Self, X11Error> {
        let (connection, screen) = x11rb::connect(None)?;
        let root = connection.setup().roots.get(screen).map_or(0, |s| s.root);
        Ok(Self { connection, root })
    }

    fn atom(&self, name: &str) -> Result<u32, X11Error> {
        Ok(self
            .connection
            .intern_atom(false, name.as_bytes())?
            .reply()?
            .atom)
    }

    pub fn prepare(&self, window: &gtk::Window) -> Result<(), X11Error> {
        WidgetExt::realize(window);
        let id = window_id(window)?;
        let kind = self.atom("_NET_WM_WINDOW_TYPE")?;
        let utility = self.atom("_NET_WM_WINDOW_TYPE_UTILITY")?;
        self.connection.change_property32(
            PropMode::REPLACE,
            id,
            kind,
            AtomEnum::ATOM,
            &[utility],
        )?;
        skip_taskbar(window);
        self.connection.flush()?;
        Ok(())
    }

    fn keep_above(&self, id: u32) -> Result<(), X11Error> {
        let state = self.atom("_NET_WM_STATE")?;
        let above = self.atom("_NET_WM_STATE_ABOVE")?;
        let event =
            ClientMessageEvent::new(32, id, state, [STATE_ADD, above, 0, SOURCE_APPLICATION, 0]);
        let mask = EventMask::SUBSTRUCTURE_REDIRECT | EventMask::SUBSTRUCTURE_NOTIFY;
        self.connection.send_event(false, self.root, mask, event)?;
        Ok(())
    }

    pub fn place(
        &self,
        window: &gtk::Window,
        area: Rect,
        click: (i32, i32),
    ) -> Result<(), X11Error> {
        let id = window_id(window)?;
        let scale = window.scale_factor();
        let size = (window.width() * scale, window.height() * scale);
        let (x, y) = window_position(area.scaled(scale), click, size);
        self.keep_above(id)?;
        self.connection
            .configure_window(id, &ConfigureWindowAux::new().x(x).y(y))?;
        self.connection.flush()?;
        Ok(())
    }
}

pub fn place_after_map(
    placer: std::rc::Rc<X11Placer>,
    window: &gtk::Window,
    area: Rect,
    click: (i32, i32),
) {
    let window = window.clone();
    glib::idle_add_local_once(move || {
        if let Err(error) = placer.place(&window, area, click) {
            tracing::warn!(%error, "could not move the popup next to the tray icon");
        }
    });
}
