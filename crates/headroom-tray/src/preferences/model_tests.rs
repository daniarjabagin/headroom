use serde_json::json;

use super::*;

#[test]
fn missing_fields_take_the_daemon_defaults() {
    let settings = settings_from(&json!({})).unwrap();
    assert_eq!(settings, Settings::default());
    assert_eq!(settings.refresh_interval_secs, 300);
    assert!(settings.notifications.almost_out);
    assert!(!settings.notifications.reset);
    assert!(settings.updates.check);
    assert!(!settings.display.combine_accounts);
}

#[test]
fn reads_every_field_and_ignores_unknown_ones() {
    let raw = json!({
        "refresh_interval_secs": 600,
        "notifications": {"almost_out": false, "reset": true},
        "headline": {"mode": "pinned", "account_id": "codex:1", "window": "weekly"},
        "reduced_motion": true,
        "display": {
            "theme": "dark", "language": "ru", "value_mode": "used", "reset_format": "exact",
            "show_trend": false, "combine_accounts": true, "translucent": true,
            "hidden_windows": {"codex:1": ["session"]}
        },
        "updates": {"check": false},
        "future": 1
    });
    let settings = settings_from(&raw).unwrap();
    assert_eq!(settings.refresh_interval_secs, 600);
    assert!(!settings.notifications.almost_out);
    assert!(settings.notifications.reset);
    assert_eq!(
        settings.headline,
        Headline::Pinned {
            account_id: "codex:1".into(),
            window: "weekly".into()
        }
    );
    assert!(settings.reduced_motion);
    assert_eq!(settings.display.theme, Theme::Dark);
    assert_eq!(settings.display.language, Language::Ru);
    assert_eq!(settings.display.value_mode, ValueMode::Used);
    assert_eq!(settings.display.reset_format, ResetFormat::Exact);
    assert!(!settings.display.show_trend);
    assert!(settings.display.combine_accounts);
    assert!(settings.display.is_window_hidden("codex:1", "session"));
    assert!(!settings.display.is_window_hidden("codex:1", "weekly"));
    assert!(!settings.updates.check);
}

#[test]
fn an_incomplete_pin_means_auto() {
    for headline in [
        json!({"mode": "pinned", "account_id": "codex:1"}),
        json!({"mode": "pinned", "account_id": "", "window": "weekly"}),
        json!({"mode": "auto", "account_id": "codex:1", "window": "weekly"}),
        json!({}),
    ] {
        let settings = settings_from(&json!({ "headline": headline })).unwrap();
        assert_eq!(settings.headline, Headline::Auto, "{headline}");
    }
}

#[test]
fn decoding_rejects_non_objects() {
    assert!(matches!(
        decode_settings("[1]"),
        Err(SettingsError::NotAnObject)
    ));
    assert!(matches!(decode_settings("{"), Err(SettingsError::Json(_))));
    assert!(decode_settings("{}").is_ok());
}

#[test]
fn merge_patch_follows_rfc_7386() {
    let mut target = json!({"a": {"b": 1, "c": 2}, "d": [1, 2], "e": 3});
    merge_patch(
        &mut target,
        &json!({"a": {"b": null, "x": {"y": 1}}, "d": [3], "e": null, "f": "new"}),
    );
    assert_eq!(
        target,
        json!({"a": {"c": 2, "x": {"y": 1}}, "d": [3], "f": "new"})
    );
    let mut scalar = json!(1);
    merge_patch(&mut scalar, &json!({"a": 1}));
    assert_eq!(scalar, json!({"a": 1}));
}

#[test]
fn hidden_windows_toggle_one_id() {
    let settings = settings_from(&json!({
        "display": {"hidden_windows": {"codex:1": ["session", "weekly"]}}
    }))
    .unwrap();
    let display = &settings.display;
    assert_eq!(
        display.hidden_windows_after("codex:1", "session", false),
        ["weekly"]
    );
    assert_eq!(
        display.hidden_windows_after("codex:1", "session", true),
        ["weekly", "session"]
    );
    assert_eq!(
        display.hidden_windows_after("claude:2", "weekly", true),
        ["weekly"]
    );
}
