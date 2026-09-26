use serde_json::json;

use super::*;
use crate::settings::{HeadlineMode, Theme};

fn customized() -> Settings {
    Settings::parse(
        &json!({
            "refresh_interval_secs": 120,
            "notifications": { "reset": true, "almost_out": false },
            "headline": { "mode": "pinned", "account_id": "codex:a", "window": "session" },
            "display": {
                "theme": "dark",
                "show_trend": false,
                "hidden_windows": { "codex:a": ["weekly"], "claude:b": ["session"] }
            }
        })
        .to_string(),
    )
    .unwrap()
}

#[test]
fn merge_patch_follows_rfc_7386_examples() {
    let cases = [
        (json!({"a":"b"}), json!({"a":"c"}), json!({"a":"c"})),
        (json!({"a":"b"}), json!({"b":"c"}), json!({"a":"b","b":"c"})),
        (json!({"a":"b"}), json!({"a":null}), json!({})),
        (
            json!({"a":"b","b":"c"}),
            json!({"a":null}),
            json!({"b":"c"}),
        ),
        (json!({"a":["b"]}), json!({"a":"c"}), json!({"a":"c"})),
        (json!({"a":"c"}), json!({"a":["b"]}), json!({"a":["b"]})),
        (
            json!({"a":{"b":"c"}}),
            json!({"a":{"b":"d","c":null}}),
            json!({"a":{"b":"d"}}),
        ),
        (json!({"a":[{"b":"c"}]}), json!({"a":[1]}), json!({"a":[1]})),
        (json!(["a", "b"]), json!(["c", "d"]), json!(["c", "d"])),
        (json!({"a":"b"}), json!(["c"]), json!(["c"])),
        (json!({"a":"foo"}), json!(null), json!(null)),
        (json!({"e":null}), json!({"a":1}), json!({"e":null,"a":1})),
        (json!([1, 2]), json!({"a":"b","c":null}), json!({"a":"b"})),
        (
            json!({}),
            json!({"a":{"bb":{"ccc":null}}}),
            json!({"a":{"bb":{}}}),
        ),
    ];
    for (mut target, patch, expected) in cases {
        merge_patch(&mut target, &patch);
        assert_eq!(target, expected, "{patch}");
    }
}

#[test]
fn patch_changes_only_the_given_fields() {
    let patched = customized()
        .patched(r#"{"display":{"translucent":true},"notifications":{"reset":false}}"#)
        .unwrap();
    let mut expected = customized();
    expected.display.translucent = true;
    expected.notifications.reset = false;
    assert_eq!(patched, expected);
}

#[test]
fn combine_accounts_is_patched_and_reset_on_its_own() {
    let on = customized()
        .patched(r#"{"display":{"combine_accounts":true}}"#)
        .unwrap();
    let mut expected = customized();
    expected.display.combine_accounts = true;
    assert_eq!(on, expected);
    let off = on
        .patched(r#"{"display":{"combine_accounts":null}}"#)
        .unwrap();
    assert_eq!(off, customized());
    assert!(
        customized()
            .patched(r#"{"display":{"combine_accounts":"yes"}}"#)
            .is_err()
    );
}

#[test]
fn null_resets_a_field_or_section_to_its_default() {
    let base = customized();
    let interval = base.patched(r#"{"refresh_interval_secs":null}"#).unwrap();
    assert_eq!(interval.refresh_interval_secs, 300);
    let theme = base.patched(r#"{"display":{"theme":null}}"#).unwrap();
    assert_eq!(theme.display.theme, Theme::System);
    assert!(!theme.display.show_trend);
    let hidden = base
        .patched(r#"{"display":{"show_breakdown":false}}"#)
        .unwrap();
    assert!(!hidden.display.show_breakdown);
    let restored = hidden
        .patched(r#"{"display":{"show_breakdown":null}}"#)
        .unwrap();
    assert!(restored.display.show_breakdown);
    let display = base.patched(r#"{"display":null}"#).unwrap();
    assert_eq!(display.display, Settings::default().display);
    assert_eq!(display.refresh_interval_secs, 120);
    assert_eq!(
        base.patched(r#"{"headline":null}"#).unwrap().headline,
        HeadlineMode::Auto {}
    );
}

#[test]
fn hidden_windows_merge_per_account() {
    let base = customized();
    let removed = base
        .patched(r#"{"display":{"hidden_windows":{"codex:a":null}}}"#)
        .unwrap();
    assert!(!removed.display.hidden_windows.contains_key("codex:a"));
    assert_eq!(removed.display.hidden_windows["claude:b"], ["session"]);
    let replaced = base
        .patched(r#"{"display":{"hidden_windows":{"codex:a":["session"],"new:c":["x"]}}}"#)
        .unwrap();
    assert_eq!(replaced.display.hidden_windows["codex:a"], ["session"]);
    assert_eq!(replaced.display.hidden_windows["claude:b"], ["session"]);
    assert_eq!(replaced.display.hidden_windows["new:c"], ["x"]);
}

#[test]
fn headline_is_replaced_whole() {
    let auto = customized()
        .patched(r#"{"headline":{"mode":"auto"}}"#)
        .unwrap();
    assert_eq!(auto.headline, HeadlineMode::Auto {});
    let pinned = Settings::default()
        .patched(r#"{"headline":{"mode":"pinned","account_id":"claude:b","window":"weekly"}}"#)
        .unwrap();
    assert_eq!(
        pinned.headline,
        HeadlineMode::Pinned {
            account_id: "claude:b".into(),
            window: "weekly".into()
        }
    );
}

#[test]
fn invalid_patches_and_results_are_rejected() {
    let base = customized();
    for patch in [
        "not json",
        r#"{"colour":"red"}"#,
        r#"{"display":{"compact":true}}"#,
        r#"{"display":{"theme":"sepia"}}"#,
        r#"{"headline":{"mode":"pinned","account_id":"a"}}"#,
    ] {
        assert!(
            matches!(base.patched(patch), Err(SettingsError::Json(_))),
            "{patch}"
        );
    }
    for patch in ["null", "[]", "3", r#""x""#] {
        assert!(
            matches!(base.patched(patch), Err(SettingsError::PatchNotObject)),
            "{patch}"
        );
    }
    assert!(matches!(
        base.patched(r#"{"refresh_interval_secs":5}"#),
        Err(SettingsError::RefreshInterval(5))
    ));
    assert!(matches!(
        base.patched(r#"{"display":{"hidden_windows":{"codex:a":[" "]}}}"#),
        Err(SettingsError::BlankHiddenWindow)
    ));
}

#[test]
fn deleting_an_unknown_field_is_a_no_op() {
    let base = customized();
    assert_eq!(base.patched(r#"{"colour":null}"#).unwrap(), base);
}

fn with_new_keys() -> Settings {
    Settings::parse(
        &json!({
            "notifications": {
                "threshold_percent": 20,
                "provider_thresholds": { "claude": 20, "copilot": 0 },
                "quiet_hours": { "enabled": true, "from": "23:00", "to": "07:00" }
            },
            "display": {
                "panel_limits": [
                    { "account_id": "codex:a", "window": "session" },
                    { "account_id": "claude:b", "window": "weekly" }
                ],
                "panel_position": { "box": "left", "index": 3 },
                "starred_accounts": ["codex:a", "claude:b"]
            }
        })
        .to_string(),
    )
    .unwrap()
}

#[test]
fn provider_thresholds_merge_per_provider() {
    let base = with_new_keys();
    let patched = base
        .patched(r#"{"notifications":{"provider_thresholds":{"claude":null,"cursor":30}}}"#)
        .unwrap();
    let thresholds = &patched.notifications.provider_thresholds;
    assert!(!thresholds.contains_key("claude"));
    assert_eq!(thresholds["copilot"], 0);
    assert_eq!(thresholds["cursor"], 30);
    assert_eq!(patched.notifications.threshold_percent, 20);
    let cleared = base
        .patched(r#"{"notifications":{"provider_thresholds":null}}"#)
        .unwrap();
    assert!(cleared.notifications.provider_thresholds.is_empty());
    assert!(matches!(
        base.patched(r#"{"notifications":{"provider_thresholds":{"cursor":60}}}"#),
        Err(SettingsError::ProviderThreshold { value: 60, .. })
    ));
}

#[test]
fn quiet_hours_merge_as_an_object() {
    let base = with_new_keys();
    let patched = base
        .patched(r#"{"notifications":{"quiet_hours":{"to":"06:30","allow_critical":false}}}"#)
        .unwrap();
    let quiet = patched.notifications.quiet_hours;
    assert!(quiet.enabled);
    assert_eq!(quiet.from.to_string(), "23:00");
    assert_eq!(quiet.to.to_string(), "06:30");
    assert!(!quiet.allow_critical);
    let from_reset = base
        .patched(r#"{"notifications":{"quiet_hours":{"from":null}}}"#)
        .unwrap();
    assert_eq!(
        from_reset.notifications.quiet_hours.from.to_string(),
        "22:00"
    );
    assert!(matches!(
        base.patched(r#"{"notifications":{"quiet_hours":{"to":"23:00"}}}"#),
        Err(SettingsError::EmptyQuietHours)
    ));
}

#[test]
fn panel_limits_and_starred_accounts_are_replaced_whole() {
    let base = with_new_keys();
    let patched = base
        .patched(
            r#"{"display":{"panel_limits":[{"account_id":"grok:c","window":"weekly"}],"starred_accounts":[]}}"#,
        )
        .unwrap();
    assert_eq!(
        patched.display.panel_limits,
        [crate::settings::PanelLimit {
            account_id: "grok:c".into(),
            window: "weekly".into()
        }]
    );
    assert!(patched.display.starred_accounts.is_empty());
    let reset = base
        .patched(r#"{"display":{"panel_limits":null}}"#)
        .unwrap();
    assert!(reset.display.panel_limits.is_empty());
    assert_eq!(reset.display.starred_accounts, ["codex:a", "claude:b"]);
}

#[test]
fn panel_position_is_replaced_whole() {
    let base = with_new_keys();
    let moved = base
        .patched(r#"{"display":{"panel_position":{"box":"center","index":0}}}"#)
        .unwrap();
    assert_eq!(
        serde_json::to_value(moved.display.panel_position).unwrap(),
        json!({ "box": "center", "index": 0 })
    );
    assert!(matches!(
        base.patched(r#"{"display":{"panel_position":{"index":1}}}"#),
        Err(SettingsError::Json(_))
    ));
    let reset = base
        .patched(r#"{"display":{"panel_position":null}}"#)
        .unwrap();
    assert_eq!(
        reset.display.panel_position,
        Settings::default().display.panel_position
    );
    assert_eq!(reset.display.panel_limits, base.display.panel_limits);
}

#[test]
fn new_top_level_sections_patch_and_reset() {
    let base = with_new_keys();
    let patched = base
        .patched(
            r#"{"adaptive_refresh":false,"status_pages":{"enabled":true},"shortcuts":{"open":"<Super>u"},"logging":{"level":"error"},"onboarding":{"completed":true}}"#,
        )
        .unwrap();
    assert!(!patched.adaptive_refresh);
    assert!(patched.status_pages.enabled);
    assert_eq!(patched.shortcuts.open, "<Super>u");
    assert_eq!(patched.notifications, base.notifications);
    let reset = patched
        .patched(r#"{"adaptive_refresh":null,"shortcuts":null,"logging":{"level":null}}"#)
        .unwrap();
    assert!(reset.adaptive_refresh);
    assert!(reset.shortcuts.open.is_empty());
    assert_eq!(reset.logging, Settings::default().logging);
    assert!(reset.onboarding.completed);
    assert!(matches!(
        base.patched(r#"{"shortcuts":{"open":"Ctrl+U"}}"#),
        Err(SettingsError::InvalidShortcut(_))
    ));
}
