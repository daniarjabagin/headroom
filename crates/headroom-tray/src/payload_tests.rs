use super::*;

const FULL: &str = include_str!("../../headroom-daemon/src/state/snapshots/state_full.json");
const EMPTY: &str = include_str!("../../headroom-daemon/src/state/snapshots/state_empty.json");
const SAMPLE: &str = include_str!("../../../shell/gnome/dev/sample-state.json");
const COMBINED: &str =
    include_str!("../../headroom-daemon/src/state/snapshots/state_combined.json");

#[test]
fn balances_accept_money_and_skip_unknown_kinds() {
    let money: Balance = serde_json::from_str(
        r#"{"id":"balance_cny","label":"Balance","kind":"money","currency":"CNY","micros":12500000}"#,
    )
    .unwrap();
    assert_eq!(
        money.amount,
        BalanceAmount::Money {
            currency: "CNY".into(),
            micros: 12_500_000
        }
    );
    let future: Balance =
        serde_json::from_str(r#"{"id":"x","label":"X","kind":"points","points":3}"#).unwrap();
    assert_eq!(future.amount, BalanceAmount::Unknown);
}

#[test]
fn parses_the_combined_snapshot() {
    let state = parse_state(COMBINED).unwrap();
    let group = &state.combined[0];
    assert_eq!(group.account_ids, ["codex:work", "codex:personal"]);
    assert_eq!(group.windows[0].segments.len(), 2);
    assert!((group.windows[0].capacity_percent - 200.0).abs() < 1e-9);
    let headline = state.headline.unwrap();
    assert!(!headline.combined);
    assert_eq!(headline.account_count, Some(1));
    assert!(parse_state(FULL).unwrap().combined.is_empty());
}

#[test]
fn headline_tolerates_a_missing_account_id() {
    let mut state: serde_json::Value = serde_json::from_str(COMBINED).unwrap();
    state["headline"]["account_id"] = serde_json::Value::Null;
    let parsed = parse_state(&state.to_string()).unwrap();
    assert_eq!(parsed.headline.unwrap().account_id, None);
    let real = parse_state(COMBINED).unwrap().headline.unwrap();
    assert_eq!(real.account_id.as_deref(), Some("claude:main"));
}

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
    assert!(state.app_version.is_some_and(|version| !version.is_empty()));
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
