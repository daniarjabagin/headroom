pub mod accelerator;
mod portal;
mod x11;

use std::sync::{Arc, Mutex, PoisonError};

use gtk::glib;

use accelerator::{Accelerator, parse};
use portal::PortalShortcut;
use x11::X11Grabber;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backend {
    Portal,
    X11,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Support {
    #[default]
    Checking,
    Supported,
    Unsupported,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SessionEnv {
    pub session_type: Option<String>,
    pub wayland_display: bool,
    pub x_display: bool,
}

impl SessionEnv {
    #[must_use]
    pub fn current() -> Self {
        let set = |name: &str| std::env::var_os(name).is_some_and(|value| !value.is_empty());
        Self {
            session_type: std::env::var("XDG_SESSION_TYPE").ok(),
            wayland_display: set("WAYLAND_DISPLAY"),
            x_display: set("DISPLAY"),
        }
    }
}

#[must_use]
pub fn backend(env: &SessionEnv) -> Option<Backend> {
    let session = env.session_type.as_deref();
    if env.wayland_display || session == Some("wayland") {
        Some(Backend::Portal)
    } else if env.x_display || session == Some("x11") {
        Some(Backend::X11)
    } else {
        None
    }
}

enum Runner {
    Portal(PortalShortcut),
    X11(X11Grabber),
}

impl Runner {
    fn start(
        backend: Backend,
        presses: async_channel::Sender<()>,
        support: &Arc<Mutex<Support>>,
    ) -> Option<Self> {
        let started = match backend {
            Backend::Portal => PortalShortcut::start(presses, Arc::clone(support))
                .map(Runner::Portal)
                .map_err(|error| error.to_string()),
            Backend::X11 => X11Grabber::start(presses)
                .map(Runner::X11)
                .map_err(|error| error.to_string()),
        };
        match started {
            Ok(runner) => Some(runner),
            Err(error) => {
                tracing::warn!(%error, ?backend, "global shortcuts are unavailable");
                None
            }
        }
    }

    fn bind(&self, accelerator: Option<Accelerator>) {
        match self {
            Runner::Portal(portal) => portal.bind(accelerator),
            Runner::X11(grabber) => {
                if let Err(error) = grabber.bind(accelerator.as_ref()) {
                    tracing::warn!(%error, "could not grab the global shortcut");
                }
            }
        }
    }
}

#[derive(Default)]
pub struct Shortcuts {
    runner: Option<Runner>,
    support: Arc<Mutex<Support>>,
    applied: Option<String>,
}

impl Shortcuts {
    pub fn start(on_press: impl Fn() + 'static) -> Self {
        let support = Arc::new(Mutex::new(Support::Checking));
        let (presses, received) = async_channel::unbounded();
        glib::spawn_future_local(async move {
            while received.recv().await.is_ok() {
                on_press();
            }
        });
        let runner = backend(&SessionEnv::current())
            .and_then(|backend| Runner::start(backend, presses, &support));
        let initial = match &runner {
            None => Support::Unsupported,
            Some(Runner::X11(_)) => Support::Supported,
            Some(Runner::Portal(_)) => Support::Checking,
        };
        let mut current = support.lock().unwrap_or_else(PoisonError::into_inner);
        if *current == Support::Checking {
            *current = initial;
        }
        drop(current);
        Self {
            runner,
            support,
            applied: None,
        }
    }

    #[must_use]
    pub fn support(&self) -> Support {
        *self.support.lock().unwrap_or_else(PoisonError::into_inner)
    }

    pub fn apply(&mut self, text: &str) {
        if self.applied.as_deref() == Some(text) {
            return;
        }
        self.applied = Some(text.to_owned());
        let Some(runner) = &self.runner else {
            return;
        };
        let accelerator = parse(text).unwrap_or_else(|error| {
            tracing::warn!(%error, "ignoring the global shortcut");
            None
        });
        runner.bind(accelerator);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env(session: Option<&str>, wayland: bool, x: bool) -> SessionEnv {
        SessionEnv {
            session_type: session.map(str::to_owned),
            wayland_display: wayland,
            x_display: x,
        }
    }

    #[test]
    fn wayland_sessions_use_the_portal_even_with_xwayland() {
        assert_eq!(
            backend(&env(Some("wayland"), true, true)),
            Some(Backend::Portal)
        );
        assert_eq!(backend(&env(None, true, true)), Some(Backend::Portal));
        assert_eq!(
            backend(&env(Some("wayland"), false, false)),
            Some(Backend::Portal)
        );
    }

    #[test]
    fn x11_sessions_grab_keys() {
        assert_eq!(backend(&env(Some("x11"), false, true)), Some(Backend::X11));
        assert_eq!(backend(&env(None, false, true)), Some(Backend::X11));
    }

    #[test]
    fn no_display_means_no_shortcut() {
        assert_eq!(backend(&env(Some("tty"), false, false)), None);
        assert_eq!(backend(&SessionEnv::default()), None);
    }

    #[test]
    fn a_manager_without_backend_remembers_the_last_setting() {
        let mut shortcuts = Shortcuts::default();
        assert_eq!(shortcuts.support(), Support::Checking);
        shortcuts.apply("<Super>u");
        shortcuts.apply("<Super>u");
        assert_eq!(shortcuts.applied.as_deref(), Some("<Super>u"));
    }
}
