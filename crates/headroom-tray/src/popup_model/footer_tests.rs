use jiff::tz::TimeZone;

use super::*;
use crate::payload::{Refresh, RefreshReason, Status, parse_state};

const SAMPLE: &str = include_str!("../../../../shell/gnome/dev/sample-state.json");

fn at(text: &str) -> Timestamp {
    text.parse().unwrap()
}

fn locale() -> Locale {
    Locale::new(Lang::En, TimeZone::UTC)
}

fn sample() -> State {
    let mut state = parse_state(SAMPLE).unwrap();
    for account in &mut state.accounts {
        account.status = Status::Fresh;
        account.refresh = None;
    }
    state.offline = false;
    state.last_success_at = Some(at("2026-09-23T09:59:00Z"));
    state.next_refresh_at = Some(at("2026-09-23T10:04:00Z"));
    state
}

fn lines(state: State) -> FooterLines {
    footer_lines(
        &View::Ready(Box::new(state)),
        &locale(),
        at("2026-09-23T10:00:00Z"),
    )
}

#[test]
fn idle_shows_the_last_and_next_update() {
    let lines = lines(sample());
    assert_eq!(lines.first.as_deref(), Some("Updated 1m ago"));
    assert_eq!(lines.second, "Next update in 4m");
    assert_eq!(lines.mood, FooterMood::Plain);
    assert_eq!(
        lines.tooltip.as_deref(),
        Some("Updated 09:59 · next at 10:04")
    );
}

#[test]
fn live_names_the_active_tool() {
    let mut state = sample();
    let name = state.accounts[0].provider_name.clone();
    state.accounts[0].refresh = Some(Refresh {
        mode: RefreshMode::Live,
        interval_secs: 60,
        next_at: None,
        reason: RefreshReason::Activity,
    });
    let lines = lines(state);
    assert_eq!(lines.mood, FooterMood::Live);
    assert_eq!(lines.second, format!("Live — every 1m while {name} runs"));
    assert!(lines.tooltip.unwrap().starts_with(&name));
}

#[test]
fn stale_and_offline_warn() {
    let mut state = sample();
    state.offline = true;
    state.last_success_at = Some(at("2026-09-23T09:48:00Z"));
    state.next_refresh_at = Some(at("2026-09-23T10:00:45Z"));
    let lines = lines(state);
    assert_eq!(lines.mood, FooterMood::Stale);
    assert_eq!(lines.first.as_deref(), Some("Outdated · updated 12m ago"));
    assert_eq!(lines.second, "Offline — retrying in 45s");
    let mut outdated = sample();
    outdated.accounts[0].status = Status::Stale;
    let lines = super::tests::lines(outdated);
    assert_eq!(lines.mood, FooterMood::Stale);
    assert_eq!(lines.second, "Next update in 4m");
}

#[test]
fn service_states() {
    let now = at("2026-09-23T10:00:00Z");
    assert_eq!(
        footer_lines(&View::Loading, &locale(), now).second,
        "Connecting…"
    );
    let stopped = View::Unavailable {
        starting: false,
        error: None,
    };
    assert_eq!(
        footer_lines(&stopped, &locale(), now).second,
        "Service not running"
    );
}
