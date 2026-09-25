use clap::Parser;

use super::*;
use crate::cli::{Cli, Command};
use crate::render::fixtures::full_state;

fn multi(state: &StatePayload, flags: &[&str]) -> WaybarLine {
    let parsed = Cli::try_parse_from([&["headroom", "waybar"], flags].concat()).unwrap();
    match parsed.command {
        Command::Waybar(args) => state_line(state, state.generated_at, &args),
        other => panic!("parsed {other:?}"),
    }
}

const AMBER_45: &str = "<span color=\"#ffd60a\">45%</span>";
const RED_8: &str = "<span color=\"#ff453a\">8%</span>";

#[test]
fn providers_show_one_toned_value_each_in_order() {
    let state = full_state();
    let line = multi(&state, &["--providers", "claude,codex"]);
    assert_eq!(line.text, format!("Claude {RED_8} · Codex {AMBER_45}"));
    assert_eq!(line.class, "critical");
    assert_eq!(line.percentage, 8);
    let compact = multi(&state, &["--providers", "codex", "--labels", "none"]);
    assert_eq!(compact.text, AMBER_45);
    assert_eq!(compact.class, "warning");
    assert_eq!(compact.percentage, 45);
}

#[test]
fn good_values_keep_the_default_color() {
    let mut state = full_state();
    state.accounts[0].windows[0].tone = Tone::Good;
    let line = multi(&state, &["--providers", "codex"]);
    assert_eq!((line.text.as_str(), line.class), ("Codex 45%", "good"));
}

#[test]
fn without_providers_every_visible_provider_is_shown() {
    let line = multi(&full_state(), &["--labels", "none"]);
    assert_eq!(line.text, format!("{AMBER_45} · {RED_8}"));
}

#[test]
fn missing_windows_show_a_dash_and_stay_neutral() {
    let state = full_state();
    let weekly = multi(
        &state,
        &["--providers", "codex,claude", "--window", "weekly"],
    );
    assert_eq!(weekly.text, "Codex — · Claude —");
    assert_eq!((weekly.class, weekly.percentage), ("neutral", 0));
    let unknown = multi(&state, &["--providers", "grok"]);
    assert_eq!(unknown.text, "grok —");
    let mut empty = full_state();
    empty.accounts.clear();
    assert_eq!(multi(&empty, &["--labels", "full"]).text, "—");
}

#[test]
fn the_lowest_window_of_a_provider_is_shown() {
    let mut state = full_state();
    state.accounts[0].windows[1].hidden = false;
    state.accounts[0].windows[1].remaining_percent = 30.0;
    let any = multi(&state, &["--providers", "codex"]);
    assert_eq!(any.text, "Codex 30%");
    let session = multi(&state, &["--providers", "codex", "--window", "session"]);
    assert_eq!(session.text, format!("Codex {AMBER_45}"));
}

#[test]
fn multi_tooltip_has_bold_headers_and_toned_values() {
    let state = full_state();
    let line = multi(&state, &["--providers", "codex"]);
    let expected = "\
<b>Codex · Work (Pro)</b>
  Session: <span color=\"#ffd60a\">45% left</span> · resets in 2h 0m · <span color=\"#ffd60a\">~8% spare</span>
<b>Claude · ada@claude.example (Pro)</b>
  sign-in expired, open the CLI to sign in again
  Session: <span color=\"#ff453a\">8% left</span> · resets in 30m · <span color=\"#ff453a\">limit in 23m</span>

Today: $0.01 · 6.2K tokens
Yesterday: $0.00 · 615 tokens (partial)
30 days: $0.01 · 6.8K tokens (partial)";
    assert_eq!(line.tooltip, expected);
}

#[test]
fn multi_escapes_user_text_but_not_its_own_markup() {
    let mut state = full_state();
    state.accounts[0].label = Some("R&D <main>".into());
    state.accounts[0].provider_name = "Co<dex> & co".into();
    state.accounts[0].windows[0].label = "<Session>".into();
    let line = multi(&state, &["--providers", "codex"]);
    assert_eq!(line.text, format!("Co&lt;dex&gt; &amp; co {AMBER_45}"));
    let header =
        "<b>Co&lt;dex&gt; &amp; co · R&amp;D &lt;main&gt; (Pro)</b>\n  &lt;Session&gt;: <span";
    assert!(line.tooltip.starts_with(header), "{}", line.tooltip);
}

#[test]
fn multi_values_follow_the_value_mode() {
    let mut state = full_state();
    state.display.value_mode = ValueMode::Used;
    let line = multi(&state, &["--providers", "codex", "--labels", "none"]);
    assert_eq!(line.text, "<span color=\"#ffd60a\">55%</span>");
    assert_eq!(line.percentage, 45);
    assert!(
        line.tooltip
            .contains("Session: <span color=\"#ffd60a\">55% used</span>")
    );
    let headline = state_line(&state, state.generated_at, &WaybarArgs::default());
    assert_eq!(headline.text, "8%");
    assert!(headline.tooltip.contains("Session: 45% left"));
}

#[test]
fn headline_drives_text_class_and_percentage() {
    let state = full_state();
    let line = state_line(&state, state.generated_at, &WaybarArgs::default());
    assert_eq!(line.text, "8%");
    assert_eq!(line.class, "critical");
    assert_eq!(line.percentage, 8);
}

#[test]
fn tooltip_summarises_visible_accounts_and_spend() {
    let state = full_state();
    let line = state_line(&state, state.generated_at, &WaybarArgs::default());
    let expected = "\
Codex · Work (Pro)
  Session: 45% left · resets in 2h 0m · ~8% spare
Claude · ada@claude.example (Pro)
  sign-in expired, open the CLI to sign in again
  Session: 8% left · resets in 30m · limit in 23m

Today: $0.01 · 6.2K tokens
Yesterday: $0.00 · 615 tokens (partial)
30 days: $0.01 · 6.8K tokens (partial)";
    assert_eq!(line.tooltip, expected);
}

#[test]
fn serializes_to_the_waybar_json_shape() {
    let state = full_state();
    let json = serde_json::to_value(state_line(
        &state,
        state.generated_at,
        &WaybarArgs::default(),
    ))
    .unwrap();
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
    let line = state_line(&state, state.generated_at, &WaybarArgs::default());
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
    let line = state_line(&state, state.generated_at, &WaybarArgs::default());
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
    assert_eq!(
        state_line(&state, state.generated_at, &WaybarArgs::default()).percentage,
        63
    );
    if let Some(headline) = state.headline.as_mut() {
        headline.remaining_percent = 140.0;
    }
    let line = state_line(&state, state.generated_at, &WaybarArgs::default());
    assert_eq!((line.text.as_str(), line.percentage), ("140%", 100));
}
