use headroom_core::pace::Tone;

use super::payload::StatePayload;
use super::tests::{assemble_sample, sample_model};
use crate::settings::{PanelLimit, PanelMode};

const FULL: &str = include_str!("snapshots/state_full.json");

#[test]
fn assembled_state_resolves_pinned_panel_limits() {
    let mut model = sample_model();
    model.settings.display.panel_mode = PanelMode::Several;
    model.settings.display.panel_limits = vec![
        PanelLimit {
            account_id: "codex:work".into(),
            window: "session".into(),
        },
        PanelLimit {
            account_id: "codex:hidden".into(),
            window: "session".into(),
        },
    ];
    let payload = assemble_sample(&model);
    let items: Vec<_> = payload
        .panel_items
        .iter()
        .map(|i| (i.headline.account_id.as_str(), i.headline.window.as_str()))
        .collect();
    assert_eq!(items, [("codex:work", "session")]);
    assert_eq!(payload.panel_tone, Some(Tone::Critical));
}

#[test]
fn assembled_state_collapses_every_unstarred_account() {
    let mut model = sample_model();
    model.settings.display.collapse_unstarred = true;
    let payload = assemble_sample(&model);
    let collapsed: Vec<_> = payload
        .accounts
        .iter()
        .map(|a| (a.id.as_str(), a.collapsed))
        .collect();
    assert_eq!(
        collapsed,
        [
            ("codex:work", true),
            ("claude:main", true),
            ("codex:hidden", true)
        ]
    );
    assert!(payload.accounts[1].error.is_some());
    model.settings.display.starred_accounts = vec!["codex:hidden".into()];
    let starred = assemble_sample(&model);
    let collapsed: Vec<_> = starred.accounts.iter().map(|a| a.collapsed).collect();
    assert_eq!(collapsed, [true, true, false]);
}

#[test]
fn payloads_without_panel_fields_still_parse() {
    let mut json: serde_json::Value = serde_json::from_str(FULL).unwrap();
    let state = json.as_object_mut().unwrap();
    state.remove("panel_items").unwrap();
    state.remove("panel_tone").unwrap();
    for account in state["accounts"].as_array_mut().unwrap() {
        account
            .as_object_mut()
            .unwrap()
            .remove("collapsed")
            .unwrap();
    }
    let parsed: StatePayload = serde_json::from_value(json).unwrap();
    assert!(parsed.panel_items.is_empty());
    assert_eq!(parsed.panel_tone, None);
    assert!(parsed.accounts.iter().all(|a| !a.collapsed));
}
