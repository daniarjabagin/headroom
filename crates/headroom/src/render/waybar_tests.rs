use super::*;
use crate::render::fixtures::full_state;

#[test]
fn headline_drives_text_class_and_percentage() {
    let state = full_state();
    let line = state_line(&state, state.generated_at);
    assert_eq!(line.text, "8%");
    assert_eq!(line.class, "critical");
    assert_eq!(line.percentage, 8);
}

#[test]
fn tooltip_summarises_visible_accounts_and_spend() {
    let state = full_state();
    let line = state_line(&state, state.generated_at);
    let expected = "\
Codex · Work (Pro)
  Session: 45% left · resets in 2h 0m · ~8% spare
  Weekly: 70% left · resets in 3d 0h
Claude Code · ada@claude.example (Pro)
  sign-in expired, open the CLI to sign in again
  Session: 8% left · resets in 30m · limit in 23m

Today: $0.00 · 1.2K tokens
Yesterday: $0.00 · 615 tokens (partial)
30 days: $0.00 · 1.8K tokens (partial)";
    assert_eq!(line.tooltip, expected);
}

#[test]
fn serializes_to_the_waybar_json_shape() {
    let state = full_state();
    let json = serde_json::to_value(state_line(&state, state.generated_at)).unwrap();
    let keys: Vec<&str> = json
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(keys, ["class", "percentage", "text", "tooltip"]);
    assert_eq!(json["percentage"], 8);
}

#[test]
fn tooltip_escapes_pango_markup() {
    let mut state = full_state();
    state.accounts[0].label = Some("R&D <main>".into());
    let line = state_line(&state, state.generated_at);
    assert!(
        line.tooltip
            .starts_with("Codex · R&amp;D &lt;main&gt; (Pro)")
    );
}

#[test]
fn no_headline_is_neutral() {
    let mut state = full_state();
    state.headline = None;
    state.accounts.clear();
    state.usage.clear();
    let line = state_line(&state, state.generated_at);
    assert_eq!(line.text, "—");
    assert_eq!(line.class, "neutral");
    assert_eq!(line.percentage, 0);
    assert_eq!(line.tooltip, "No limits reported yet");
}

#[test]
fn absent_daemon_is_neutral() {
    let line = absent_line();
    assert_eq!(line.class, "neutral");
    assert_eq!(line.tooltip, "Headroom daemon is not running");
}

#[test]
fn percentage_is_clamped_and_rounded() {
    let mut state = full_state();
    if let Some(headline) = state.headline.as_mut() {
        headline.remaining_percent = 62.5;
    }
    assert_eq!(state_line(&state, state.generated_at).percentage, 63);
    if let Some(headline) = state.headline.as_mut() {
        headline.remaining_percent = 140.0;
    }
    let line = state_line(&state, state.generated_at);
    assert_eq!((line.text.as_str(), line.percentage), ("140%", 100));
}
