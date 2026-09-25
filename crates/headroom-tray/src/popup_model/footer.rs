use jiff::Timestamp;

use crate::account::{HeaderStatus, header_status};
use crate::dates::{Locale, clock_time};
use crate::format::{ago_text, duration, millis_between, next_update_text};
use crate::i18n::{Lang, fill};
use crate::payload::{Account, RefreshMode, State};
use crate::view::View;

const SECOND_MS: i64 = 1000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FooterMood {
    Plain,
    Live,
    Stale,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FooterLines {
    pub first: Option<String>,
    pub second: String,
    pub mood: FooterMood,
    pub tooltip: Option<String>,
}

impl FooterLines {
    fn plain(second: String) -> Self {
        Self {
            first: None,
            second,
            mood: FooterMood::Plain,
            tooltip: None,
        }
    }
}

fn visible(state: &State) -> impl Iterator<Item = &Account> {
    state.accounts.iter().filter(|account| !account.hidden)
}

fn is_stale(state: &State) -> bool {
    state.offline
        || visible(state)
            .any(|account| header_status(account, state.offline) == Some(HeaderStatus::Outdated))
}

fn updated(lang: Lang, state: &State, now: Timestamp) -> Option<String> {
    let at = state.last_success_at?;
    Some(fill(
        lang.tr("Updated {ago}"),
        &[("ago", &ago_text(lang, at, now))],
    ))
}

fn next_line(lang: Lang, state: &State, now: Timestamp) -> String {
    if state.is_refreshing() {
        return lang.tr("Updating…").to_owned();
    }
    state
        .next_refresh_at
        .map(|next| next_update_text(lang, next, now))
        .unwrap_or_default()
}

fn stale_lines(locale: &Locale, state: &State, now: Timestamp) -> FooterLines {
    let lang = locale.lang;
    let first = state.last_success_at.map_or_else(
        || lang.tr("Outdated").to_owned(),
        |at| {
            fill(
                lang.tr("Outdated · updated {ago}"),
                &[("ago", &ago_text(lang, at, now))],
            )
        },
    );
    let retry = state
        .next_refresh_at
        .map(|next| millis_between(now, next))
        .filter(|left| *left > 0);
    let second = match (state.offline, retry) {
        (true, Some(left)) => fill(
            lang.tr("Offline — retrying in {duration}"),
            &[("duration", &duration(lang, left.max(SECOND_MS), true))],
        ),
        (true, None) => lang.tr("Offline").to_owned(),
        (false, _) => next_line(lang, state, now),
    };
    let tooltip = state.last_success_at.map(|at| {
        fill(
            lang.tr("No successful update since {time}. The numbers are the last ones Headroom received."),
            &[("time", &clock_time(at, locale))],
        )
    });
    FooterLines {
        first: Some(first),
        second,
        mood: FooterMood::Stale,
        tooltip,
    }
}

fn live_tools(state: &State) -> (Vec<&str>, Option<u32>) {
    let mut names: Vec<&str> = Vec::new();
    let mut interval: Option<u32> = None;
    for account in visible(state) {
        let Some(refresh) = account.refresh.as_ref() else {
            continue;
        };
        if refresh.mode != RefreshMode::Live {
            continue;
        }
        if !names.contains(&account.provider_name.as_str()) {
            names.push(&account.provider_name);
        }
        interval = Some(interval.map_or(refresh.interval_secs, |i| i.min(refresh.interval_secs)));
    }
    (names, interval)
}

fn live_lines(lang: Lang, state: &State, now: Timestamp) -> Option<FooterLines> {
    let (names, interval) = live_tools(state);
    let interval = interval?;
    let tool = names.join(", ");
    let every = duration(lang, i64::from(interval) * SECOND_MS, false);
    let values = [("interval", every.as_str()), ("tool", tool.as_str())];
    let second = if state.is_refreshing() {
        lang.tr("Updating…").to_owned()
    } else {
        fill(
            lang.tr("Live — every {interval} while {tool} runs"),
            &values,
        )
    };
    Some(FooterLines {
        first: updated(lang, state, now),
        second,
        mood: FooterMood::Live,
        tooltip: Some(fill(
            lang.tr("{tool} wrote to its local logs in the last 10 minutes, so it is checked every {interval}. Back to the normal interval after 10 minutes without activity."),
            &values,
        )),
    })
}

fn idle_lines(locale: &Locale, state: &State, now: Timestamp) -> FooterLines {
    let lang = locale.lang;
    let tooltip = state
        .last_success_at
        .zip(state.next_refresh_at)
        .map(|(done, next)| {
            fill(
                lang.tr("Updated {updated} · next at {next}"),
                &[
                    ("updated", &clock_time(done, locale)),
                    ("next", &clock_time(next, locale)),
                ],
            )
        });
    FooterLines {
        first: updated(lang, state, now),
        second: next_line(lang, state, now),
        mood: FooterMood::Plain,
        tooltip,
    }
}

#[must_use]
pub fn footer_lines(view: &View, locale: &Locale, now: Timestamp) -> FooterLines {
    let lang = locale.lang;
    match view {
        View::Unavailable { .. } => FooterLines::plain(lang.tr("Service not running").into()),
        View::Loading => FooterLines::plain(lang.tr("Connecting…").into()),
        View::Failed(_) => FooterLines::plain(String::new()),
        View::Ready(state) if is_stale(state) => stale_lines(locale, state, now),
        View::Ready(state) => {
            live_lines(lang, state, now).unwrap_or_else(|| idle_lines(locale, state, now))
        }
    }
}

#[cfg(test)]
#[path = "footer_tests.rs"]
mod tests;
