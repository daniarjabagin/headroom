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
