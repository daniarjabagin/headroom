use super::*;

const FULL: &str = include_str!("../../headroom-daemon/src/state/snapshots/state_full.json");
const EMPTY: &str = include_str!("../../headroom-daemon/src/state/snapshots/state_empty.json");
const SAMPLE: &str = include_str!("../../../shell/gnome/dev/sample-state.json");

#[test]
fn parses_the_daemon_snapshots() {
    let full = parse_state(FULL).unwrap();
    assert!(!full.accounts.is_empty());
    assert!(!full.usage.is_empty());
    let empty = parse_state(EMPTY).unwrap();
    assert!(empty.accounts.is_empty());
    assert!(empty.headline.is_none());
}

#[test]
fn parses_the_gnome_sample_state() {
    let state = parse_state(SAMPLE).unwrap();
    assert_eq!(state.app_version.as_deref(), Some("0.4.1"));
    assert_eq!(state.display.value_mode, ValueMode::Left);
}

#[test]
fn rejects_another_schema_version() {
    let json = FULL.replacen("\"version\": 1", "\"version\": 2", 1);
    assert!(matches!(parse_state(&json), Err(PayloadError::Version(2))));
}

#[test]
fn unknown_enum_values_fall_back() {
    let tone: Tone = serde_json::from_str("\"purple\"").unwrap();
    assert_eq!(tone, Tone::Neutral);
    let known: Tone = serde_json::from_str("\"critical\"").unwrap();
    assert_eq!(known, Tone::Critical);
    let neutral: Tone = serde_json::from_str("\"neutral\"").unwrap();
    assert_eq!(neutral, Tone::Neutral);
    let system: Theme = serde_json::from_str("\"system\"").unwrap();
    assert_eq!(system, Theme::System);
    let theme: Theme = serde_json::from_str("\"dark\"").unwrap();
    assert_eq!(theme, Theme::Dark);
    let status: Status = serde_json::from_str("\"signed_out\"").unwrap();
    assert_eq!(status, Status::SignedOut);
    let install: InstallKind = serde_json::from_str("\"self\"").unwrap();
    assert_eq!(install, InstallKind::SelfManaged);
}

#[test]
fn display_defaults_fill_missing_fields() {
    let display: Display = serde_json::from_str(r#"{"theme":"light"}"#).unwrap();
    assert_eq!(display.theme, Theme::Light);
    assert!(display.show_spend);
}

#[test]
fn finds_the_usage_of_an_account() {
    let state = parse_state(FULL).unwrap();
    let with_usage = state
        .accounts
        .iter()
        .find(|account| state.usage_of(account).is_some());
    assert!(with_usage.is_some());
}
