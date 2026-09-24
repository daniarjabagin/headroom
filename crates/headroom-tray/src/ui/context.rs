use std::cell::RefCell;
use std::collections::BTreeSet;
use std::rc::Rc;

use jiff::Timestamp;

use crate::dates::Locale;
use crate::palette::{Palette, Rgba};
use crate::payload::{Display, Tone};
use crate::spend::Period;
use crate::update::UpdateRun;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    RefreshNow,
    Refresh(String),
    ToggleValueMode,
    ToggleResetFormat,
    SelectPeriod(Period),
    SetExpanded(String, bool),
    StartService,
    InstallUpdate,
    ToggleUpdateCommand,
    Copy(String),
    OpenUrl(String),
    SignIn(String),
    Quit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RefreshMode {
    #[default]
    Idle,
    Busy,
    Failed,
}

#[derive(Debug, Clone, Default)]
pub struct UiState {
    pub period: Period,
    pub expanded: BTreeSet<String>,
    pub refresh: RefreshMode,
    pub update_run: UpdateRun,
    pub update_command_open: bool,
    pub copied: bool,
    pub service_starting: bool,
    pub service_error: Option<String>,
}

const FALLBACK: Rgba = Rgba {
    red: 0.5,
    green: 0.5,
    blue: 0.5,
    alpha: 1.0,
};

pub type Tick = Box<dyn Fn(Timestamp)>;

pub struct Ctx {
    pub locale: Locale,
    pub display: Display,
    pub palette: Palette,
    pub motion: bool,
    pub offline: bool,
    pub sign_in: BTreeSet<String>,
    pub ui: UiState,
    pub version: String,
    pub act: Rc<dyn Fn(Action)>,
    pub ticks: RefCell<Vec<Tick>>,
}

impl Ctx {
    pub fn action(&self, action: Action) -> impl Fn() + 'static {
        let act = Rc::clone(&self.act);
        move || act(action.clone())
    }

    pub fn on_tick(&self, tick: impl Fn(Timestamp) + 'static) {
        self.ticks.borrow_mut().push(Box::new(tick));
    }

    pub fn color(&self, token: &str) -> Rgba {
        self.palette.color(token).unwrap_or(FALLBACK)
    }

    pub fn tone_color(&self, tone: Tone) -> Rgba {
        self.palette.tone(tone).unwrap_or(FALLBACK)
    }

    pub fn series_color(&self, provider: &str) -> Rgba {
        self.palette.series(provider).unwrap_or(FALLBACK)
    }

    pub fn css(&self, token: &str) -> String {
        self.palette
            .css_value(token)
            .map_or_else(|_| String::from("#808080"), str::to_owned)
    }
}
