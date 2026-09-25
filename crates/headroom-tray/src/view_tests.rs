use jiff::tz::TimeZone;

use super::*;
use crate::payload::{Tone, parse_state};

const SAMPLE: &str = include_str!("../../../shell/gnome/dev/sample-state.json");

fn at(text: &str) -> Timestamp {
    text.parse().unwrap()
}

fn locale() -> Locale {
    Locale::new(Lang::En, TimeZone::UTC)
}

fn sample() -> State {
    parse_state(SAMPLE).unwrap()
}

#[test]
fn tray_look_uses_the_headline() {
    let view = View::Ready(Box::new(sample()));
    let look = tray_look(&view, &locale(), at("2026-09-23T10:00:00Z"));
    assert_eq!(look.tooltip, "Codex · Session 62% left · resets in 2h 41m");
    let ring = look.ring.unwrap();
    assert_eq!((ring.percent, ring.tone), (62, Tone::Good));
}

#[test]
fn tray_look_without_data_shows_the_status() {
    let unavailable = View::Unavailable {
        starting: false,
        error: None,
    };
    let look = tray_look(&unavailable, &locale(), at("2026-09-23T10:00:00Z"));
    assert_eq!(look.ring, None);
    assert_eq!(look.tooltip, "Headroom service isn't running");
    let mut state = sample();
    state.headline = None;
    let look = tray_look(
        &View::Ready(Box::new(state)),
        &locale(),
        at("2026-09-23T10:00:00Z"),
    );
    assert_eq!(look.tooltip, "No usage limits to show");
}

#[test]
fn combined_headline_names_the_account_count() {
    let mut state = sample();
    if let Some(headline) = state.headline.as_mut() {
        headline.combined = true;
        headline.account_count = Some(2);
        headline.account_label = None;
    }
    let look = tray_look(
        &View::Ready(Box::new(state)),
        &locale(),
        at("2026-09-23T10:00:00Z"),
    );
    assert!(look.tooltip.starts_with("Codex ×2 · Session"));
}
