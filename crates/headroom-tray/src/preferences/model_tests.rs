use serde_json::json;

use super::*;
use crate::payload::{
    Density, Language, PanelBox, PanelIndicator, PanelLabel, PanelMode, PanelPosition, ResetFormat,
    SpendBreakdown, SpendPeriod, SpendUnit, Theme, TimeFormat, ValueMode,
};
use crate::preferences::change::Change;

#[test]
fn missing_fields_take_the_daemon_defaults() {
    let settings = settings_from(&json!({})).unwrap();
    assert_eq!(settings, Settings::default());
    assert_eq!(settings.refresh_interval_secs, 300);
    assert!(settings.notifications.almost_out);
    assert!(!settings.notifications.reset);
    assert!(settings.updates.check);
    assert!(!settings.display.combine_accounts);
    assert!(settings.display.show_breakdown);
}

#[test]
fn the_breakdown_toggle_reads_the_daemon_value() {
    let hidden = settings_from(&json!({"display": {"show_breakdown": false}})).unwrap();
    assert!(!hidden.display.show_breakdown);
    let older = settings_from(&json!({"display": {"show_spend": false}})).unwrap();
    assert!(older.display.show_breakdown);
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

#[test]
fn release_0_6_keys_take_documented_defaults() {
    let settings = Settings::default();
    assert!(settings.adaptive_refresh);
    assert_eq!(settings.notifications.threshold_percent, 10);
    assert!(settings.notifications.provider_thresholds.is_empty());
    assert_eq!(settings.notifications.quiet_hours, QuietHours::default());
    assert_eq!(settings.notifications.quiet_hours.from.to_string(), "22:00");
    assert_eq!(settings.notifications.quiet_hours.to.to_string(), "08:00");
    assert!(settings.notifications.quiet_hours.allow_critical);
    assert!(!settings.status_pages.enabled);
    assert_eq!(settings.shortcuts.open, "");
    assert_eq!(settings.logging.level, LogLevel::Info);
    assert!(!settings.onboarding.completed);
    let display = &settings.display;
    assert_eq!(display.density, Density::Normal);
    assert_eq!(display.time_format, TimeFormat::Auto);
    assert_eq!(display.panel_mode, PanelMode::Headline);
    assert_eq!(display.panel_indicator, PanelIndicator::Ring);
    assert_eq!(display.panel_label, PanelLabel::Percent);
    assert_eq!(display.panel_position.panel_box, PanelBox::Right);
    assert_eq!(display.panel_position.index, 0);
    assert_eq!(display.spend_period, SpendPeriod::Last30Days);
    assert_eq!(display.spend_unit, SpendUnit::Cost);
    assert_eq!(display.spend_breakdown, SpendBreakdown::Models);
    assert!(display.hide_on_screen_share);
    assert!(!display.collapse_unstarred);
}

#[test]
fn reads_the_documented_0_6_document() {
    let raw = json!({
        "adaptive_refresh": false,
        "notifications": {
            "threshold_percent": 20,
            "provider_thresholds": {"claude": 20, "copilot": 0},
            "quiet_hours": {"enabled": true, "from": "23:00", "to": "07:30", "allow_critical": false}
        },
        "display": {
            "density": "compact", "time_format": "24h", "panel_mode": "several",
            "panel_indicator": "bar", "panel_label": "none",
            "panel_limits": [{"account_id": "claude:1", "window": "session"}],
            "panel_position": {"box": "center", "index": 2},
            "spend_period": "7d", "spend_unit": "tokens", "spend_breakdown": "projects",
            "starred_accounts": ["claude:1"], "collapse_unstarred": true,
            "hide_on_screen_share": false
        },
        "status_pages": {"enabled": true},
        "shortcuts": {"open": "<Super>u"},
        "logging": {"level": "debug"},
        "onboarding": {"completed": true}
    });
    let settings = settings_from(&raw).unwrap();
    assert!(!settings.adaptive_refresh);
    let notifications = &settings.notifications;
    assert_eq!(notifications.threshold_for("claude"), 20);
    assert_eq!(notifications.threshold_for("copilot"), 0);
    assert_eq!(notifications.threshold_for("codex"), 20);
    assert!(notifications.quiet_hours.enabled);
    assert_eq!(notifications.quiet_hours.to, ClockTime::new(7, 30).unwrap());
    assert!(!notifications.quiet_hours.allow_critical);
    let display = &settings.display;
    assert_eq!(display.density, Density::Compact);
    assert_eq!(display.time_format, TimeFormat::H24);
    assert_eq!(display.panel_label, PanelLabel::None);
    assert_eq!(display.panel_limits.len(), 1);
    assert_eq!(display.panel_position.panel_box, PanelBox::Center);
    assert_eq!(display.spend_period, SpendPeriod::Last7Days);
    assert_eq!(display.spend_unit, SpendUnit::Tokens);
    assert!(display.is_starred("claude:1"));
    assert!(settings.status_pages.enabled);
    assert_eq!(settings.shortcuts.open, "<Super>u");
    assert_eq!(settings.logging.level, LogLevel::Debug);
    assert!(settings.onboarding.completed);
}

#[test]
fn unknown_values_fall_back_to_defaults() {
    let raw = json!({
        "display": {"density": "cozy", "time_format": "36h", "panel_mode": "wall"},
        "logging": {"level": "trace"}
    });
    let settings = settings_from(&raw).unwrap();
    assert_eq!(settings.display.density, Density::Normal);
    assert_eq!(settings.display.time_format, TimeFormat::Auto);
    assert_eq!(settings.display.panel_mode, PanelMode::Headline);
    assert_eq!(settings.logging.level, LogLevel::Info);
}

fn merged(stored: serde_json::Value, change: &Change) -> Settings {
    let mut raw = stored;
    merge_patch(&mut raw, &change.patch());
    settings_from(&raw).unwrap()
}

#[test]
fn provider_thresholds_merge_one_provider_at_a_time() {
    let stored = json!({"notifications": {"provider_thresholds": {"claude": 20, "copilot": 0}}});
    let set = merged(
        stored.clone(),
        &Change::ProviderThreshold {
            provider: "codex".into(),
            threshold: Some(5),
        },
    );
    assert_eq!(set.notifications.provider_thresholds.len(), 3);
    let cleared = merged(
        stored,
        &Change::ProviderThreshold {
            provider: "claude".into(),
            threshold: None,
        },
    );
    let keys: Vec<&String> = cleared.notifications.provider_thresholds.keys().collect();
    assert_eq!(keys, ["copilot"]);
}

#[test]
fn quiet_hours_and_panel_position_replace_every_field() {
    let stored = json!({"notifications": {"quiet_hours": {"enabled": true, "from": "21:00"}},
                        "display": {"panel_position": {"box": "left", "index": 4}}});
    let quiet = merged(
        stored.clone(),
        &Change::QuietHours(QuietHours {
            allow_critical: false,
            ..QuietHours::default()
        }),
    );
    assert_eq!(
        quiet.notifications.quiet_hours,
        QuietHours {
            allow_critical: false,
            ..QuietHours::default()
        }
    );
    let moved = merged(
        stored,
        &Change::PanelPosition(PanelPosition {
            panel_box: PanelBox::Right,
            index: 1,
        }),
    );
    assert_eq!(moved.display.panel_position.panel_box, PanelBox::Right);
    assert_eq!(moved.display.panel_position.index, 1);
}
