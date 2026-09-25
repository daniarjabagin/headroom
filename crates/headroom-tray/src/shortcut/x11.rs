use std::sync::{Arc, Mutex, PoisonError};
use std::thread;

use gtk::gdk;
use gtk::glib::translate::IntoGlib;
use x11rb::connection::Connection;
use x11rb::protocol::Event;
use x11rb::protocol::xproto::{ConnectionExt, GrabMode, ModMask};
use x11rb::rust_connection::RustConnection;

use super::accelerator::{Accelerator, Modifiers};

const SHIFT: u16 = 1;
const LOCK: u16 = 1 << 1;
const CONTROL: u16 = 1 << 2;
const MOD1: u16 = 1 << 3;
const NUM_LOCK: u16 = 1 << 4;
const MOD4: u16 = 1 << 6;
const IGNORED: u16 = LOCK | NUM_LOCK;
const RELEVANT: u16 = SHIFT | CONTROL | MOD1 | MOD4;
const REPEAT_WINDOW_MS: u32 = 250;

#[derive(Debug, thiserror::Error)]
pub enum X11ShortcutError {
    #[error("could not connect to the X server: {0}")]
    Connect(#[from] x11rb::errors::ConnectError),
    #[error("X11 request failed: {0}")]
    Connection(#[from] x11rb::errors::ConnectionError),
    #[error("X11 reply failed: {0}")]
    Reply(#[from] x11rb::errors::ReplyError),
    #[error("the key {0} is not on this keyboard")]
    UnknownKey(String),
    #[error("could not start the shortcut listener: {0}")]
    Thread(#[from] std::io::Error),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Grab {
    pub keycode: u8,
    pub mask: u16,
}

#[must_use]
pub fn modifier_mask(modifiers: Modifiers) -> u16 {
    [
        (modifiers.shift, SHIFT),
        (modifiers.control, CONTROL),
        (modifiers.alt, MOD1),
        (modifiers.super_key, MOD4),
    ]
    .iter()
    .filter(|(on, _)| *on)
    .fold(0, |mask, (_, bit)| mask | bit)
}

#[must_use]
pub fn lock_variants(mask: u16) -> [u16; 4] {
    [mask, mask | LOCK, mask | NUM_LOCK, mask | LOCK | NUM_LOCK]
}

#[must_use]
pub fn keycode_for(keysym: u32, min_keycode: u8, per_keycode: u8, keysyms: &[u32]) -> Option<u8> {
    let row = keysyms
        .chunks(usize::from(per_keycode.max(1)))
        .position(|row| row.contains(&keysym))?;
    u8::try_from(row).ok()?.checked_add(min_keycode)
}

#[must_use]
pub fn matches(grab: Grab, keycode: u8, state: u16) -> bool {
    grab.keycode == keycode && state & !IGNORED & RELEVANT == grab.mask
}

#[must_use]
pub fn is_repeat(last: Option<u32>, now: u32) -> bool {
    last.is_some_and(|last| now.wrapping_sub(last) < REPEAT_WINDOW_MS)
}

fn keysym(key: &str) -> Option<u32> {
    gdk::Key::from_name(key).map(|key| key.to_lower().into_glib())
}

pub struct X11Grabber {
    connection: Arc<RustConnection>,
    root: u32,
    grab: Arc<Mutex<Option<Grab>>>,
}

fn listen(
    connection: &RustConnection,
    grab: &Mutex<Option<Grab>>,
    presses: &async_channel::Sender<()>,
) {
    let mut last = None;
    while let Ok(event) = connection.wait_for_event() {
        let Event::KeyPress(press) = event else {
            continue;
        };
        let current = *grab.lock().unwrap_or_else(PoisonError::into_inner);
        let hit = current.is_some_and(|grab| matches(grab, press.detail, u16::from(press.state)));
        if hit && !is_repeat(last, press.time) && presses.try_send(()).is_err() {
            return;
        }
        if hit {
            last = Some(press.time);
        }
    }
}

impl X11Grabber {
    pub fn start(presses: async_channel::Sender<()>) -> Result<Self, X11ShortcutError> {
        let (connection, screen) = RustConnection::connect(None)?;
        let root = connection.setup().roots[screen].root;
        let connection = Arc::new(connection);
        let grab = Arc::new(Mutex::new(None));
        let (listener, grabbed) = (Arc::clone(&connection), Arc::clone(&grab));
        thread::Builder::new()
            .name("headroom-shortcut".into())
            .spawn(move || listen(&listener, &grabbed, &presses))?;
        Ok(Self {
            connection,
            root,
            grab,
        })
    }

    fn resolve(&self, accelerator: &Accelerator) -> Result<Grab, X11ShortcutError> {
        let unknown = || X11ShortcutError::UnknownKey(accelerator.key.clone());
        let keysym = keysym(&accelerator.key).ok_or_else(unknown)?;
        let setup = self.connection.setup();
        let (min, max) = (setup.min_keycode, setup.max_keycode);
        let mapping = self
            .connection
            .get_keyboard_mapping(min, max.saturating_sub(min).saturating_add(1))?
            .reply()?;
        let keycode = keycode_for(keysym, min, mapping.keysyms_per_keycode, &mapping.keysyms)
            .ok_or_else(unknown)?;
        Ok(Grab {
            keycode,
            mask: modifier_mask(accelerator.modifiers),
        })
    }

    fn ungrab(&self, grab: Grab) -> Result<(), X11ShortcutError> {
        for mask in lock_variants(grab.mask) {
            self.connection
                .ungrab_key(grab.keycode, self.root, ModMask::from(mask))?;
        }
        self.connection.flush()?;
        Ok(())
    }

    fn grab(&self, grab: Grab) -> Result<(), X11ShortcutError> {
        for mask in lock_variants(grab.mask) {
            self.connection
                .grab_key(
                    false,
                    self.root,
                    ModMask::from(mask),
                    grab.keycode,
                    GrabMode::ASYNC,
                    GrabMode::ASYNC,
                )?
                .check()?;
        }
        Ok(())
    }

    pub fn bind(&self, accelerator: Option<&Accelerator>) -> Result<(), X11ShortcutError> {
        let mut current = self.grab.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(old) = current.take() {
            self.ungrab(old)?;
        }
        let Some(accelerator) = accelerator else {
            return Ok(());
        };
        let wanted = self.resolve(accelerator)?;
        if let Err(error) = self.grab(wanted) {
            self.ungrab(wanted)?;
            return Err(error);
        }
        *current = Some(wanted);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modifiers_map_to_core_masks() {
        let all = Modifiers {
            shift: true,
            control: true,
            alt: true,
            super_key: true,
        };
        assert_eq!(modifier_mask(all), SHIFT | CONTROL | MOD1 | MOD4);
        assert_eq!(modifier_mask(Modifiers::default()), 0);
        assert_eq!(lock_variants(MOD4), [64, 66, 80, 82]);
    }

    #[test]
    fn finds_the_keycode_in_the_mapping() {
        let keysyms = [0x61, 0x41, 0x75, 0x55, 0xffbe, 0];
        assert_eq!(keycode_for(0x75, 8, 2, &keysyms), Some(9));
        assert_eq!(keycode_for(0x55, 8, 2, &keysyms), Some(9));
        assert_eq!(keycode_for(0x62, 8, 2, &keysyms), None);
    }

    #[test]
    fn presses_match_despite_lock_keys() {
        let grab = Grab {
            keycode: 30,
            mask: MOD4,
        };
        assert!(matches(grab, 30, MOD4 | NUM_LOCK | LOCK));
        assert!(!matches(grab, 30, MOD4 | SHIFT));
        assert!(!matches(grab, 31, MOD4));
    }

    #[test]
    fn auto_repeat_is_ignored() {
        assert!(!is_repeat(None, 100));
        assert!(is_repeat(Some(100), 200));
        assert!(!is_repeat(Some(100), 400));
        assert!(is_repeat(Some(u32::MAX - 10), 50));
    }

    #[test]
    fn key_names_resolve_to_lowercase_keysyms() {
        assert_eq!(keysym("u"), Some(0x75));
        assert_eq!(keysym("U"), Some(0x75));
        assert_eq!(keysym("F5"), Some(0xffc2));
        assert_eq!(keysym("space"), Some(0x20));
        assert_eq!(keysym("nonsense_key"), None);
    }
}
