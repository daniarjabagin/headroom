use super::*;

const EMPTY: &str = include_str!("snapshots/state_empty.json");
const FULL: &str = include_str!("snapshots/state_full.json");

#[test]
fn payloads_from_daemons_without_combined_accounts_still_parse() {
    let mut json: serde_json::Value = serde_json::from_str(FULL).unwrap();
    let state = json.as_object_mut().unwrap();
    state.remove("combined").unwrap();
    let headline = state["headline"].as_object_mut().unwrap();
    headline.remove("combined").unwrap();
    headline.remove("account_count").unwrap();
    let parsed: StatePayload = serde_json::from_value(json).unwrap();
    assert!(parsed.combined.is_empty());
    let headline = parsed.headline.unwrap();
    assert!(!headline.combined);
    assert_eq!(headline.account_count, 1);
}

#[test]
fn the_app_version_is_the_daemon_release() {
    assert_eq!(APP_VERSION, env!("CARGO_PKG_VERSION"));
}

#[test]
fn payloads_from_daemons_without_an_app_version_still_parse() {
    let mut json: serde_json::Value = serde_json::from_str(EMPTY).unwrap();
    json.as_object_mut().unwrap().remove("app_version").unwrap();
    let parsed: StatePayload = serde_json::from_value(json).unwrap();
    assert_eq!(parsed.app_version, None);
    assert_eq!(parsed.version, STATE_VERSION);
}

#[test]
fn payloads_from_daemons_without_update_check_still_parse() {
    let mut json: serde_json::Value = serde_json::from_str(FULL).unwrap();
    json.as_object_mut()
        .unwrap()
        .remove("update_check")
        .unwrap();
    let parsed: StatePayload = serde_json::from_value(json).unwrap();
    assert_eq!(parsed.update_check, None);
    let full: StatePayload = serde_json::from_str(FULL).unwrap();
    assert_eq!(
        full.update_check,
        Some(UpdateCheckView {
            checked_at: Some("2026-09-23T04:00:00Z".parse().unwrap())
        })
    );
}
