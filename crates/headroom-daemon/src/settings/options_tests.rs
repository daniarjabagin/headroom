use serde_json::json;

use super::*;

fn parse(value: &serde_json::Value) -> Result<Settings, SettingsError> {
    Settings::parse(&value.to_string())
}

fn limit(account_id: &str, window: &str) -> PanelLimit {
    PanelLimit {
        account_id: account_id.into(),
        window: window.into(),
    }
}

#[test]
fn new_sections_round_trip() {
    let document = json!({
        "adaptive_refresh": false,
        "notifications": {
            "threshold_percent": 20,
            "provider_thresholds": { "claude": 20, "copilot": 0, "future-provider": 50 },
            "quiet_hours": {
                "enabled": true, "from": "23:30", "to": "07:05", "allow_critical": false
            }
        },
        "status_pages": { "enabled": true },
        "shortcuts": { "open": "<Super>u" },
        "logging": { "level": "debug" },
        "onboarding": { "completed": true }
    });
    let settings = parse(&document).unwrap();
    assert!(!settings.adaptive_refresh);
    assert_eq!(settings.notifications.threshold_percent, 20);
    assert_eq!(settings.notifications.provider_thresholds["copilot"], 0);
    assert!(settings.notifications.quiet_hours.enabled);
    assert!(settings.status_pages.enabled);
    assert_eq!(settings.shortcuts.open, "<Super>u");
    assert_eq!(settings.logging.level, LogLevel::Debug);
    assert!(settings.onboarding.completed);
    let back = serde_json::to_value(&settings).unwrap();
    for key in [
        "adaptive_refresh",
        "status_pages",
        "shortcuts",
        "logging",
        "onboarding",
    ] {
        assert_eq!(back[key], document[key], "{key}");
    }
    assert_eq!(
        back["notifications"]["quiet_hours"],
        document["notifications"]["quiet_hours"]
    );
    assert_eq!(
        back["notifications"]["provider_thresholds"],
        document["notifications"]["provider_thresholds"]
    );
}

#[test]
fn panel_limits_and_starred_accounts_are_deduplicated_in_order() {
    let settings = parse(&json!({ "display": {
        "panel_limits": [
            { "account_id": "codex:a", "window": "weekly" },
            { "account_id": "claude:b", "window": "session" },
            { "account_id": "codex:a", "window": "weekly" },
            { "account_id": "codex:a", "window": "session" }
        ],
        "starred_accounts": ["claude:b", "codex:a", "claude:b"]
    }}))
    .unwrap();
    assert_eq!(
        settings.display.panel_limits,
        [
            limit("codex:a", "weekly"),
            limit("claude:b", "session"),
            limit("codex:a", "session")
        ]
    );
    assert_eq!(settings.display.starred_accounts, ["claude:b", "codex:a"]);
}

#[test]
fn panel_limits_are_validated() {
    let four: Vec<_> = ["a", "b", "c", "d"]
        .iter()
        .map(|w| json!({ "account_id": "codex:a", "window": w }))
        .collect();
    assert!(matches!(
        parse(&json!({ "display": { "panel_limits": four } })),
        Err(SettingsError::TooManyPanelLimits { found: 4, max: 3 })
    ));
    for blank in [
        json!([{ "account_id": " ", "window": "session" }]),
        json!([{ "account_id": "codex:a", "window": "" }]),
    ] {
        assert!(matches!(
            parse(&json!({ "display": { "panel_limits": blank } })),
            Err(SettingsError::BlankPanelLimit)
        ));
    }
    for incomplete in [
        json!([{ "account_id": "codex:a" }]),
        json!([{ "window": "session" }]),
        json!(["codex:a"]),
    ] {
        assert!(matches!(
            parse(&json!({ "display": { "panel_limits": incomplete } })),
            Err(SettingsError::Json(_))
        ));
    }
    assert!(parse(&json!({ "display": { "panel_limits": [] } })).is_ok());
}

#[test]
fn panel_position_needs_a_known_box_and_a_non_negative_index() {
    let settings =
        parse(&json!({ "display": { "panel_position": { "box": "left", "index": 4 } } })).unwrap();
    assert_eq!(
        settings.display.panel_position,
        PanelPosition {
            panel_box: PanelBox::Left,
            index: 4
        }
    );
    for invalid in [
        json!({ "box": "left", "index": -1 }),
        json!({ "box": "left" }),
        json!({ "index": 0 }),
        json!({ "box": "left", "index": 1.5 }),
    ] {
        assert!(matches!(
            parse(&json!({ "display": { "panel_position": invalid } })),
            Err(SettingsError::Json(_))
        ));
    }
}

#[test]
fn panel_label_accepts_none() {
    let settings =
        parse(&json!({ "display": { "panel_label": "none", "panel_indicator": "none" } })).unwrap();
    assert_eq!(settings.display.panel_label, PanelLabel::None);
    assert_eq!(settings.display.panel_indicator, PanelIndicator::None);
}

#[test]
fn blank_starred_accounts_are_rejected() {
    assert!(matches!(
        parse(&json!({ "display": { "starred_accounts": ["codex:a", " "] } })),
        Err(SettingsError::BlankStarredAccount)
    ));
}

#[test]
fn threshold_percent_is_between_1_and_50() {
    for value in [1, 5, 50] {
        assert!(parse(&json!({ "notifications": { "threshold_percent": value } })).is_ok());
    }
    for value in [0, 51, 100] {
        assert!(matches!(
            parse(&json!({ "notifications": { "threshold_percent": value } })),
            Err(SettingsError::ThresholdPercent(v)) if v == value
        ));
    }
    assert!(matches!(
        parse(&json!({ "notifications": { "threshold_percent": 300 } })),
        Err(SettingsError::Json(_))
    ));
}

#[test]
fn provider_thresholds_are_between_0_and_50() {
    assert!(matches!(
        parse(&json!({ "notifications": { "provider_thresholds": { "claude": 51 } } })),
        Err(SettingsError::ProviderThreshold { provider, value: 51 }) if provider == "claude"
    ));
    assert!(matches!(
        parse(&json!({ "notifications": { "provider_thresholds": { " ": 10 } } })),
        Err(SettingsError::BlankProviderThreshold)
    ));
    assert!(matches!(
        parse(&json!({ "notifications": { "provider_thresholds": { "claude": null } } })),
        Err(SettingsError::Json(_))
    ));
}

#[test]
fn quiet_hours_times_are_strict_hh_mm() {
    for valid in ["00:00", "23:59", "08:05"] {
        let quiet = json!({ "from": valid, "to": "12:00" });
        assert!(
            parse(&json!({ "notifications": { "quiet_hours": quiet } })).is_ok(),
            "{valid}"
        );
    }
    for invalid in [
        "24:00", "12:60", "8:00", "08:0", "08-00", "08:00:00", "", "ab:cd", "+1:00",
    ] {
        let quiet = json!({ "enabled": true, "from": invalid });
        let error = parse(&json!({ "notifications": { "quiet_hours": quiet } })).unwrap_err();
        assert!(
            matches!(&error, SettingsError::Json(e) if e.to_string().contains("HH:MM")),
            "{invalid}: {error}"
        );
    }
}

#[test]
fn enabled_quiet_hours_need_distinct_ends() {
    let same = json!({ "from": "22:00", "to": "22:00" });
    assert!(parse(&json!({ "notifications": { "quiet_hours": same } })).is_ok());
    let enabled = json!({ "enabled": true, "from": "22:00", "to": "22:00" });
    assert!(matches!(
        parse(&json!({ "notifications": { "quiet_hours": enabled } })),
        Err(SettingsError::EmptyQuietHours)
    ));
}

#[test]
fn shortcut_is_empty_or_a_gtk_accelerator() {
    for valid in [
        "",
        "<Super>u",
        "<Control><Alt>F12",
        "space",
        "<Shift>Page_Up",
        "<Primary>1",
    ] {
        assert!(
            parse(&json!({ "shortcuts": { "open": valid } })).is_ok(),
            "{valid}"
        );
    }
    for invalid in [
        "<Super>",
        "<>u",
        "<Super u",
        "Super+U",
        "<Super> u",
        "<Ctrl1>u",
        "ü",
    ] {
        assert!(
            matches!(
                parse(&json!({ "shortcuts": { "open": invalid } })),
                Err(SettingsError::InvalidShortcut(text)) if text == invalid
            ),
            "{invalid}"
        );
    }
    let long = format!("<Super>{}", "a".repeat(58));
    assert!(matches!(
        parse(&json!({ "shortcuts": { "open": long } })),
        Err(SettingsError::ShortcutTooLong(64))
    ));
    let longest = format!("<Super>{}", "a".repeat(57));
    assert!(parse(&json!({ "shortcuts": { "open": longest } })).is_ok());
}

#[test]
fn stored_settings_from_before_onboarding_count_as_completed() {
    let old = Settings::from_stored(r#"{"reduced_motion":true}"#).unwrap();
    assert!(old.onboarding.completed);
    let empty = Settings::from_stored("{}").unwrap();
    assert!(empty.onboarding.completed);
    let pending = Settings::from_stored(r#"{"onboarding":{"completed":false}}"#).unwrap();
    assert!(!pending.onboarding.completed);
    assert!(!Settings::default().onboarding.completed);
}

#[test]
fn stored_settings_from_before_0_6_load_the_new_defaults() {
    let old =
        Settings::from_stored(r#"{"display":{"theme":"dark"},"notifications":{"reset":true}}"#)
            .unwrap();
    let expected = Settings::default()
        .patched(
            r#"{"display":{"theme":"dark"},"notifications":{"reset":true},"onboarding":{"completed":true}}"#,
        )
        .unwrap();
    assert_eq!(old, expected);
}

#[test]
fn saved_settings_load_back_unchanged() {
    let fresh = Settings::default()
        .patched(r#"{"display":{"panel_mode":"icon"},"logging":{"level":"warn"}}"#)
        .unwrap();
    let stored = serde_json::to_string(&fresh).unwrap();
    assert_eq!(Settings::from_stored(&stored).unwrap(), fresh);
}

#[test]
fn reset_keeps_only_onboarding() {
    let custom = Settings::default()
        .patched(
            r#"{"refresh_interval_secs":120,"adaptive_refresh":false,"onboarding":{"completed":true},
                "display":{"density":"compact","hidden_windows":{"codex:a":["weekly"]}},
                "notifications":{"provider_thresholds":{"claude":20}},"shortcuts":{"open":"<Super>u"}}"#,
        )
        .unwrap();
    let reset = custom.reset();
    assert!(reset.onboarding.completed);
    assert_eq!(
        reset,
        Settings {
            onboarding: OnboardingSettings { completed: true },
            ..Settings::default()
        }
    );
    assert_eq!(Settings::default().reset(), Settings::default());
}
