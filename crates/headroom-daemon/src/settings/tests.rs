use serde_json::json;

use super::*;

#[test]
fn defaults_match_the_spec() {
    let expected = json!({
        "refresh_interval_secs": 300,
        "notifications": {
            "almost_out": true, "cutting_it_close": true, "will_run_out": true, "reset": false
        },
        "headline": { "mode": "auto" },
        "reduced_motion": false,
        "display": {
            "theme": "system",
            "language": "system",
            "value_mode": "left",
            "reset_format": "countdown",
            "panel_label": "percent",
            "show_spend": true,
            "show_account_spend": true,
            "show_trend": true,
            "show_forecast": true,
            "translucent": false,
            "combine_accounts": false,
            "hidden_windows": {}
        },
        "updates": { "check": true }
    });
    assert_eq!(serde_json::to_value(Settings::default()).unwrap(), expected);
    assert_eq!(Settings::parse("{}").unwrap(), Settings::default());
}

#[test]
fn missing_fields_take_defaults() {
    let settings = Settings::parse(r#"{"notifications":{"reset":true}}"#).unwrap();
    assert!(settings.notifications.reset);
    assert!(settings.notifications.almost_out);
    assert_eq!(settings.refresh_interval_secs, 300);
    let display = Settings::parse(r#"{"display":{"theme":"dark"}}"#)
        .unwrap()
        .display;
    assert_eq!(display.theme, Theme::Dark);
    assert!(display.show_spend);
    assert!(!display.translucent);
    assert!(!display.combine_accounts);
}

#[test]
fn display_options_round_trip() {
    let json = json!({
        "display": {
            "theme": "light",
            "language": "ru",
            "value_mode": "used",
            "reset_format": "exact",
            "panel_label": "window",
            "show_spend": false,
            "show_account_spend": false,
            "show_trend": false,
            "show_forecast": false,
            "translucent": true,
            "combine_accounts": true,
            "hidden_windows": { "codex:abc": ["weekly", "model:spark"] }
        }
    });
    let settings = Settings::parse(&json.to_string()).unwrap();
    let display = &settings.display;
    assert_eq!(display.theme, Theme::Light);
    assert_eq!(display.language, Language::Ru);
    assert_eq!(display.value_mode, ValueMode::Used);
    assert_eq!(display.reset_format, ResetFormat::Exact);
    assert_eq!(display.panel_label, PanelLabel::Window);
    assert!(display.translucent);
    assert!(display.combine_accounts);
    assert!(display.is_hidden("codex:abc", "model:spark"));
    assert!(!display.is_hidden("codex:abc", "session"));
    assert!(!display.is_hidden("claude:x", "weekly"));
    let back = serde_json::to_value(&settings).unwrap();
    assert_eq!(back["display"], json["display"]);
}

#[test]
fn hidden_window_lists_are_deduplicated_in_order() {
    let json = r#"{"display":{"hidden_windows":{"gone:1":["weekly","session","weekly"]}}}"#;
    let settings = Settings::parse(json).unwrap();
    assert_eq!(
        settings.display.hidden_windows["gone:1"],
        ["weekly", "session"]
    );
}

#[test]
fn blank_hidden_window_ids_are_rejected() {
    for json in [
        r#"{"display":{"hidden_windows":{" ":["weekly"]}}}"#,
        r#"{"display":{"hidden_windows":{"codex:a":[""]}}}"#,
    ] {
        assert!(
            matches!(Settings::parse(json), Err(SettingsError::BlankHiddenWindow)),
            "{json}"
        );
    }
}

#[test]
fn unknown_fields_are_rejected_at_every_level() {
    for json in [
        r#"{"show_usage":true}"#,
        r#"{"colour":"red"}"#,
        r#"{"notifications":{"loud":true}}"#,
        r#"{"display":{"compact":true}}"#,
        r#"{"updates":{"install":true}}"#,
        r#"{"headline":{"mode":"auto","account_id":"codex:a"}}"#,
        r#"{"headline":{"mode":"pinned","account_id":"a","window":"w","x":1}}"#,
    ] {
        assert!(
            matches!(Settings::parse(json), Err(SettingsError::Json(_))),
            "{json}"
        );
    }
}

#[test]
fn unknown_enum_values_are_rejected() {
    for json in [
        r#"{"display":{"theme":"sepia"}}"#,
        r#"{"display":{"language":"de"}}"#,
        r#"{"display":{"value_mode":"both"}}"#,
        r#"{"display":{"reset_format":"relative"}}"#,
        r#"{"display":{"panel_label":"icon"}}"#,
    ] {
        assert!(
            matches!(Settings::parse(json), Err(SettingsError::Json(_))),
            "{json}"
        );
    }
}

#[test]
fn stored_show_usage_migrates_to_show_spend() {
    let stored = Settings::from_stored(r#"{"show_usage":false,"reduced_motion":true}"#).unwrap();
    assert!(!stored.display.show_spend);
    assert!(stored.reduced_motion);
    let explicit =
        Settings::from_stored(r#"{"show_usage":false,"display":{"show_spend":true}}"#).unwrap();
    assert!(explicit.display.show_spend);
    let current = Settings::from_stored(r#"{"display":{"show_spend":false}}"#).unwrap();
    assert!(!current.display.show_spend);
}

#[test]
fn stored_display_without_translucent_loads_the_default() {
    let stored = Settings::from_stored(r#"{"display":{"theme":"dark"}}"#).unwrap();
    assert_eq!(stored.display.theme, Theme::Dark);
    assert!(!stored.display.translucent);
    let enabled = Settings::from_stored(r#"{"display":{"translucent":true}}"#).unwrap();
    assert!(enabled.display.translucent);
}

#[test]
fn update_checks_default_on_for_settings_stored_before_them() {
    let stored = Settings::from_stored(r#"{"reduced_motion":true}"#).unwrap();
    assert!(stored.updates.check);
    let off = Settings::default()
        .patched(r#"{"updates":{"check":false}}"#)
        .unwrap();
    assert!(!off.updates.check);
    assert!(off.patched(r#"{"updates":null}"#).unwrap().updates.check);
}

#[test]
fn serializes_headline_with_mode_tag() {
    let json = serde_json::to_value(Settings::default()).unwrap();
    assert_eq!(json["headline"], json!({ "mode": "auto" }));
    let pinned = Settings::parse(
        r#"{"headline":{"mode":"pinned","account_id":"codex:abc","window":"session"}}"#,
    )
    .unwrap();
    assert_eq!(
        pinned.headline,
        HeadlineMode::Pinned {
            account_id: "codex:abc".into(),
            window: "session".into()
        }
    );
}

#[test]
fn rejects_out_of_range_interval() {
    for value in [0, 59, 3_601] {
        let json = format!(r#"{{"refresh_interval_secs":{value}}}"#);
        assert!(matches!(
            Settings::parse(&json),
            Err(SettingsError::RefreshInterval(v)) if v == value
        ));
    }
    assert!(Settings::parse(r#"{"refresh_interval_secs":60}"#).is_ok());
    assert!(Settings::parse(r#"{"refresh_interval_secs":3600}"#).is_ok());
}

#[test]
fn rejects_empty_pinned_target() {
    let json = r#"{"headline":{"mode":"pinned","account_id":" ","window":"session"}}"#;
    assert!(matches!(
        Settings::parse(json),
        Err(SettingsError::EmptyHeadlineTarget)
    ));
}

#[test]
fn rejects_malformed_json_and_wrong_types() {
    assert!(matches!(Settings::parse("{"), Err(SettingsError::Json(_))));
    assert!(matches!(
        Settings::parse(r#"{"reduced_motion":"yes"}"#),
        Err(SettingsError::Json(_))
    ));
    assert!(matches!(
        Settings::parse(r#"{"display":{"translucent":"yes"}}"#),
        Err(SettingsError::Json(_))
    ));
    assert!(matches!(
        Settings::parse(r#"{"display":{"hidden_windows":{"a":"weekly"}}}"#),
        Err(SettingsError::Json(_))
    ));
    assert!(matches!(
        Settings::parse(r#"{"headline":{"mode":"loudest"}}"#),
        Err(SettingsError::Json(_))
    ));
}

#[test]
fn daemon_managed_keys_are_ignored_on_input_and_dropped_from_storage() {
    let parsed = Settings::parse(r#"{"reduced_motion":true,"dismissed_accounts":["grok:a"]}"#);
    assert!(parsed.unwrap().reduced_motion);
    let stored = Settings::from_stored(r#"{"dismissed_accounts":["grok:a"]}"#).unwrap();
    assert_eq!(stored, Settings::default());
    let patched = Settings::default()
        .patched(r#"{"dismissed_accounts":["grok:a"],"reduced_motion":true}"#)
        .unwrap();
    assert!(patched.reduced_motion);
    let served = serde_json::to_value(&patched).unwrap();
    assert!(served.get("dismissed_accounts").is_none());
}
