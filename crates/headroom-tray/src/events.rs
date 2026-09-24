use crate::icon::Pixmap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MenuAction {
    Open,
    RefreshNow,
    Quit,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    State(String),
    Settings(String),
    Providers(String),
    Vanished,
    CallFailed(String),
    RefreshSettled(bool),
    ServiceStarted(Result<(), String>),
    OpenRequested,
    Activate { x: i32, y: i32 },
    Menu(MenuAction),
    TrayFailed(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    RefreshNow,
    Refresh(String),
    UpdateSettings(String),
    StartService,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuLabels {
    pub open: String,
    pub refresh: String,
    pub quit: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrayUpdate {
    pub ring: Option<Vec<Pixmap>>,
    pub tooltip: String,
    pub labels: MenuLabels,
}

#[derive(Debug, Clone)]
pub struct Events(async_channel::Sender<Event>);

impl Events {
    #[must_use]
    pub fn new(sender: async_channel::Sender<Event>) -> Self {
        Self(sender)
    }

    pub fn send(&self, event: Event) {
        if self.0.try_send(event).is_err() {
            tracing::debug!("the interface stopped listening for events");
        }
    }
}
