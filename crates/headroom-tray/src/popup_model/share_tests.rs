use jiff::tz::TimeZone;

use super::*;
use crate::payload::{State, parse_state};

const COMBINED: &str =
    include_str!("../../../headroom-daemon/src/state/snapshots/state_combined.json");

fn state() -> State {
    parse_state(COMBINED).unwrap()
}

fn locale(lang: Lang) -> Locale {
    Locale::new(lang, TimeZone::UTC)
}

fn input<'a>(state: &'a State, locale: &'a Locale) -> ShareInput<'a> {
    ShareInput {
        locale,
        display: &state.display,
        headline: state.headline.as_ref(),
        now: state.generated_at,
    }
}

#[test]
fn an_account_card_uses_the_headline_window() {
    let state = state();
    let locale = locale(Lang::En);
    let claude = state
        .accounts
        .iter()
        .find(|a| a.id == "claude:main")
        .unwrap();
    let card = account_card(input(&state, &locale), claude).unwrap();
    assert_eq!(card.hero, "8%");
    assert_eq!(card.hero_unit, "left");
    assert_eq!(card.sub, "session · resets in 30m");
    assert_eq!(card.meta.as_deref(), Some("Pro"));
    assert_eq!(card.stamp, "CLAUDE LIMITS · 23 SEP 2026");
    assert_eq!(card.bars, [0.08]);
    assert_eq!(card.rows.len(), 1);
}

#[test]
fn a_combined_card_picks_the_tightest_window_and_hides_labels() {
    let state = state();
    let locale = locale(Lang::En);
    let group = &state.combined[0];
    let card = group_card(input(&state, &locale), group, &state.accounts).unwrap();
    assert_eq!(card.hero, "40%");
    assert_eq!(card.sub, "of 100% weekly · resets in 2d 0h");
    assert_eq!(card.meta.as_deref(), Some("2 accounts · Pro + Plus"));
    assert_eq!(card.bars, [0.4]);
    assert_eq!(card.rows[0].segments.len(), 2);
    let text = summary_text(Lang::En, &card);
    assert!(text.starts_with("Codex limits · 23 Sep 2026\nCodex · 2 accounts · Pro + Plus\n"));
    assert!(text.contains("Session: 125% left of 200% · Resets in 2h 0m"));
    for private in ["Work", "Personal", "ada@", "codex:work"] {
        assert!(!text.contains(private), "{private}");
        assert!(!format!("{card:?}").contains(private), "{private}");
    }
}

#[test]
fn russian_dates_and_file_names() {
    let state = state();
    let ru = locale(Lang::Ru);
    let claude = state
        .accounts
        .iter()
        .find(|a| a.id == "claude:main")
        .unwrap();
    let card = account_card(input(&state, &ru), claude).unwrap();
    assert_eq!(card.date, "23.09.2026");
    assert_eq!(
        file_name("claude", &ru, state.generated_at),
        "headroom-claude-2026-09-23-1000.png"
    );
    assert_eq!(
        file_name("a/b", &ru, state.generated_at),
        "headroom-a-b-2026-09-23-1000.png"
    );
}
