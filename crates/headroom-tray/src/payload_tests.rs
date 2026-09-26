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
    let today = &state.spend.today;
    let models = today.models.as_ref().unwrap();
    assert_eq!(models[0].provider, "codex");
    assert_eq!(models[0].usage.model, "gpt-5.5");
    assert_eq!(models[0].usage.cost_per_mtok_usd_micros, Some(3_756_938));
    let other = today.models_other.as_ref().unwrap();
    assert_eq!(other.count, 7);
    assert_eq!(other.cost_per_mtok_usd_micros, Some(1_646_068));
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

#[test]
fn parses_recovery_and_update_check_from_the_daemon_snapshot() {
    let full = parse_state(FULL).unwrap();
    assert_eq!(
        full.update_check,
        Some(UpdateCheck {
            checked_at: Some("2026-09-23T04:00:00Z".parse().unwrap())
        })
    );
    assert_eq!(full.accounts[0].recovery, RecoveryField::Wait);
    assert_eq!(
        full.accounts[1].recovery,
        RecoveryField::Offered(Recovery::Retry)
    );
}

fn with_first_account(field: &str, value: serde_json::Value) -> State {
    let mut state: serde_json::Value = serde_json::from_str(FULL).unwrap();
    state["accounts"][0][field] = value;
    parse_state(&state.to_string()).unwrap()
}

#[test]
fn recovery_is_parsed_tolerantly() {
    let cases = [
        (
            serde_json::json!({"action": "cli_login", "command": "codex login"}),
            RecoveryField::Offered(Recovery::CliLogin {
                command: "codex login".into(),
                account_id: None,
            }),
        ),
        (
            serde_json::json!({"action": "sign_in", "account_id": "claude:work"}),
            RecoveryField::Offered(Recovery::SignIn {
                account_id: Some("claude:work".into()),
            }),
        ),
        (
            serde_json::json!({"action": "teleport"}),
            RecoveryField::Offered(Recovery::Unknown),
        ),
        (
            serde_json::json!({"action": "cli_login", "command": 7}),
            RecoveryField::Offered(Recovery::Unknown),
        ),
        (
            serde_json::json!(42),
            RecoveryField::Offered(Recovery::Unknown),
        ),
    ];
    for (value, expected) in cases {
        let state = with_first_account("recovery", value.clone());
        assert_eq!(state.accounts[0].recovery, expected, "{value}");
    }
    let mut old: serde_json::Value = serde_json::from_str(FULL).unwrap();
    old["accounts"][0]
        .as_object_mut()
        .unwrap()
        .remove("recovery");
    let old = parse_state(&old.to_string()).unwrap();
    assert_eq!(old.accounts[0].recovery, RecoveryField::Unreported);
}

#[test]
fn account_changed_errors_parse() {
    let state = with_first_account(
        "error",
        serde_json::json!({"kind": "account_changed", "message": "Another account"}),
    );
    let error = state.accounts[0].error.as_ref().unwrap();
    assert_eq!(error.kind, "account_changed");
}

#[test]
fn update_check_is_optional_and_tolerant() {
    let mut state: serde_json::Value = serde_json::from_str(FULL).unwrap();
    state["update_check"] = serde_json::json!({"checked_at": null});
    let parsed = parse_state(&state.to_string()).unwrap();
    assert_eq!(parsed.update_check, Some(UpdateCheck { checked_at: None }));
    state["update_check"] = serde_json::json!({"checked_at": "yesterday"});
    assert_eq!(parse_state(&state.to_string()).unwrap().update_check, None);
    state["update_check"] = serde_json::Value::Null;
    assert_eq!(parse_state(&state.to_string()).unwrap().update_check, None);
    state.as_object_mut().unwrap().remove("update_check");
    assert_eq!(parse_state(&state.to_string()).unwrap().update_check, None);
}
