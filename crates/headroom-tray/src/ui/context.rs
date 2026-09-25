use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

use jiff::Timestamp;

use crate::dates::{Clock, Locale};
use crate::palette::{Palette, Rgba};
use crate::payload::{Density, Display, SpendBreakdown, SpendPeriod, SpendUnit, Tone};
use crate::popup_model::spend_view::{SpendChoice, SpendOverride};
use crate::preferences::registry::ProviderLinks;
use crate::update::UpdateRun;
use crate::update_check::CheckRun;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    RefreshNow,
    Refresh(String),
    Retry(String),
    ToggleValueMode,
    ToggleResetFormat,
    SelectPeriod(SpendPeriod),
    SelectUnit(SpendUnit),
    SelectBreakdown(SpendBreakdown),
    SetMoreExpanded(bool),
    HideAccounts(Vec<String>),
    SetStarred(Vec<String>, bool),
    Share(ShareTarget),
    CopySummary(ShareTarget),
    SetExpanded(String, bool),
    StartService,
    InstallUpdate,
    ToggleUpdateCommand,
    Copy(String),
    OpenUrl(String),
    SignIn(String),
    SignInAgain(String),
    CopyCommand { account_id: String, command: String },
    OpenSettings,
    Quit,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShareTarget {
    Account(String),
    Combined(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RefreshMode {
    #[default]
    Idle,
    Busy,
    Failed,
}

#[derive(Debug, Clone, Default)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "each flag is independent per-session popup state"
)]
pub struct UiState {
    pub spend: SpendOverride,
    pub more_expanded: bool,
    pub system_clock: Option<Clock>,
    pub expanded: BTreeSet<String>,
    pub refresh: RefreshMode,
    pub update_run: UpdateRun,
    pub update_command_open: bool,
    pub copied: bool,
    pub service_starting: bool,
    pub service_error: Option<String>,
    pub retrying: BTreeSet<String>,
    pub copied_command: Option<String>,
    pub update_check: CheckRun,
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
    pub links: Rc<BTreeMap<String, ProviderLinks>>,
    pub spend: Option<SpendChoice>,
    pub recent: bool,
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

    #[must_use]
    pub fn compact(&self) -> bool {
        self.display.density == Density::Compact
    }

    pub fn css(&self, token: &str) -> String {
        self.palette
            .css_value(token)
            .map_or_else(|_| String::from("#808080"), str::to_owned)
    }
}
