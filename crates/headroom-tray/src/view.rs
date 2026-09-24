use jiff::Timestamp;

use crate::dates::{Locale, clock_time};
use crate::format::{
    next_update_text, percent_reading, reset_phrase, updated_at_text, window_label,
};
use crate::i18n::{Lang, fill};
use crate::icon::RingKey;
use crate::payload::{Headline, State, ValueMode};

#[derive(Debug, Clone, PartialEq, Default)]
pub enum View {
    #[default]
    Loading,
    Unavailable {
        starting: bool,
        error: Option<String>,
    },
    Failed(String),
    Ready(Box<State>),
}

impl View {
    #[must_use]
    pub fn state(&self) -> Option<&State> {
        match self {
            View::Ready(state) => Some(state),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusLine {
    pub text: String,
    pub notice: bool,
}

impl StatusLine {
    fn plain(text: String) -> Self {
        Self {
            text,
            notice: false,
        }
    }
}

#[must_use]
pub fn footer_status(view: &View, locale: &Locale, now: Timestamp) -> StatusLine {
    let lang = locale.lang;
    let state = match view {
        View::Unavailable { .. } => {
            return StatusLine::plain(lang.tr("Service not running").into());
        }
        View::Loading => return StatusLine::plain(lang.tr("Connecting…").into()),
        View::Failed(_) => return StatusLine::plain(String::new()),
        View::Ready(state) => state,
    };
    if state.offline {
        let text = state.last_success_at.map_or_else(
            || lang.tr("Offline").to_owned(),
            |at| {
                let time = clock_time(at, &locale.tz);
                fill(lang.tr("Offline — last update {time}"), &[("time", &time)])
            },
        );
        return StatusLine { text, notice: true };
    }
    if state.is_refreshing() {
        return StatusLine::plain(lang.tr("Updating…").into());
    }
    if let Some(next) = state.next_refresh_at {
        return StatusLine::plain(next_update_text(lang, next, now));
    }
    StatusLine::plain(
        state
            .last_success_at
            .map(|at| updated_at_text(locale, at))
            .unwrap_or_default(),
    )
}

#[derive(Debug, Clone, PartialEq)]
pub struct TrayLook {
    pub ring: Option<RingKey>,
    pub tooltip: String,
}

fn headline_resets_at(state: &State, headline: &Headline) -> Option<Timestamp> {
    state
        .accounts
        .iter()
        .find(|account| headline.account_id.as_deref() == Some(account.id.as_str()))?
        .windows
        .iter()
        .find(|window| window.id == headline.window)?
        .resets_at
}

fn headline_source(headline: &Headline) -> String {
    match headline.account_count {
        Some(count) if headline.combined => format!("{} ×{count}", headline.provider_name),
        _ => headline.provider_name.clone(),
    }
}

fn headline_text(state: &State, headline: &Headline, locale: &Locale, now: Timestamp) -> String {
    let display = &state.display;
    let percent = match display.value_mode {
        ValueMode::Left => headline.remaining_percent,
        ValueMode::Used => headline.used_percent,
    };
    let window = window_label(locale.lang, &headline.window, &headline.window_label);
    let reading = percent_reading(locale.lang, percent, display.value_mode);
    let base = format!("{} · {window} {reading}", headline_source(headline));
    match headline_resets_at(state, headline) {
        Some(reset) => {
            let phrase = reset_phrase(locale, reset, now, display.reset_format, false);
            format!("{base} · {phrase}")
        }
        None => base,
    }
}

fn status_tooltip(view: &View, lang: Lang) -> String {
    match view {
        View::Loading => lang.tr("Connecting…").to_owned(),
        View::Unavailable { .. } => lang.tr("Headroom service isn't running").to_owned(),
        View::Failed(_) => lang.tr("Couldn't read Headroom's state").to_owned(),
        View::Ready(_) => lang.tr("No usage limits to show").to_owned(),
    }
}

#[must_use]
pub fn tray_look(view: &View, locale: &Locale, now: Timestamp) -> TrayLook {
    let headline = view
        .state()
        .and_then(|state| state.headline.as_ref().map(|headline| (state, headline)));
    match headline {
        Some((state, headline)) => TrayLook {
            ring: Some(RingKey::from_headline(headline, state.display.value_mode)),
            tooltip: headline_text(state, headline, locale, now),
        },
        None => TrayLook {
            ring: None,
            tooltip: status_tooltip(view, locale.lang),
        },
    }
}

#[cfg(test)]
#[path = "view_tests.rs"]
mod tests;
