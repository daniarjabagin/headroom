use super::*;

const EMPTY: &str = include_str!("snapshots/state_empty.json");

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
