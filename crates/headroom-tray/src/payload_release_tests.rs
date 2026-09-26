use serde_json::{Value, json};

use super::*;

const FULL: &str = include_str!("../../headroom-daemon/src/state/snapshots/state_full.json");
const COMBINED: &str =
    include_str!("../../headroom-daemon/src/state/snapshots/state_combined.json");

fn edited(edit: impl FnOnce(&mut Value)) -> State {
    let mut state: Value = serde_json::from_str(FULL).unwrap();
    edit(&mut state);
    parse_state(&state.to_string()).unwrap()
}

fn remove(value: &mut Value, key: &str) {
    value.as_object_mut().unwrap().remove(key);
}

#[test]
fn parses_panel_items_and_tone() {
    let state = parse_state(FULL).unwrap();
    assert!(state.speaks_0_6());
    assert_eq!(state.panel_tone, Some(Tone::Critical));
    let items = state.panel_items.as_ref().unwrap();
    assert_eq!(items.len(), 1);
    let item = &items[0];
    assert_eq!(item.headline.account_id.as_deref(), Some("claude:main"));
    assert!((item.value_percent - 8.0).abs() < 1e-9);
    assert_eq!(item.even_pace_percent, Some(90.0));
    assert_eq!(item.logo(), "claude");
    assert_eq!(state.resolved_panel_items(), *items);
}

#[test]
fn older_daemons_get_one_item_from_the_headline() {
    let state = edited(|state| {
        remove(state, "panel_items");
        remove(state, "panel_tone");
        state["display"]["value_mode"] = json!("used");
    });
    assert!(!state.speaks_0_6());
    assert_eq!(state.panel_tone, None);
    let items = state.resolved_panel_items();
    assert_eq!(items.len(), 1);
    assert!((items[0].value_percent - 92.0).abs() < 1e-9);
    assert_eq!(items[0].even_pace_percent, None);
    assert_eq!(items[0].logo(), "claude");
    let empty = edited(|state| {
        remove(state, "panel_items");
        state["headline"] = Value::Null;
    });
    assert!(empty.resolved_panel_items().is_empty());
}

#[test]
fn broken_panel_items_are_skipped() {
    let state = edited(|state| {
        state["panel_items"]
            .as_array_mut()
            .unwrap()
            .push(json!({"provider": "x"}));
        state["panel_tone"] = json!(7);
    });
    assert_eq!(state.panel_items.map(|items| items.len()), Some(1));
    assert_eq!(state.panel_tone, None);
}

#[test]
fn parses_provider_status() {
    let state = parse_state(FULL).unwrap();
    let claude = state.provider_status_of("claude").unwrap();
    assert_eq!(claude.indicator, StatusIndicator::Minor);
    assert_eq!(claude.tone, Tone::Warning);
    assert_eq!(
        claude.title.as_deref(),
        Some("Elevated errors on Claude Code")
    );
    assert_eq!(claude.stage.as_deref(), Some("identified"));
    assert!(claude.started_at.is_some());
    assert!(!claude.is_clear());
    assert!(state.provider_status_of("codex").is_none());
    let tolerant = edited(|state| {
        let list = state["provider_status"].as_array_mut().unwrap();
        list.push(
            json!({"provider": "codex", "indicator": "none", "tone": "neutral",
                         "title": null, "stage": null, "started_at": null,
                         "url": "https://status.openai.com"}),
        );
        list.push(json!({"provider": "broken"}));
    });
    assert_eq!(tolerant.provider_status.len(), 2);
    assert!(tolerant.provider_status_of("codex").unwrap().is_clear());
    let old = edited(|state| remove(state, "provider_status"));
    assert!(old.provider_status.is_empty());
}

#[test]
fn parses_account_refresh_and_collapse() {
    let state = parse_state(FULL).unwrap();
    let refresh = state.accounts[0].refresh.as_ref().unwrap();
    assert_eq!(refresh.mode, RefreshMode::Idle);
    assert_eq!(refresh.interval_secs, 300);
    assert_eq!(refresh.reason, RefreshReason::Schedule);
    assert!(refresh.next_at.is_some());
    assert_eq!(
        state.accounts[1]
            .refresh
            .as_ref()
            .map(|refresh| refresh.reason),
        Some(RefreshReason::Backoff)
    );
    assert!(!state.accounts[0].collapsed);
    let live = edited(|state| {
        state["accounts"][0]["collapsed"] = json!(true);
        state["accounts"][0]["refresh"] =
            json!({"mode": "live", "interval_secs": 60, "next_at": null, "reason": "activity"});
        state["accounts"][1]["refresh"] = json!({"mode": "live"});
        remove(&mut state["accounts"][2], "refresh");
        remove(&mut state["accounts"][2], "collapsed");
    });
    assert!(live.accounts[0].collapsed);
    let refresh = live.accounts[0].refresh.as_ref().unwrap();
    assert_eq!(refresh.mode, RefreshMode::Live);
    assert_eq!(refresh.reason, RefreshReason::Activity);
    assert_eq!(live.accounts[1].refresh, None);
    assert_eq!(live.accounts[2].refresh, None);
    assert!(!live.accounts[2].collapsed);
}

#[test]
fn parses_combined_collapse() {
    let state = parse_state(COMBINED).unwrap();
    assert!(!state.combined[0].collapsed);
    let mut raw: Value = serde_json::from_str(COMBINED).unwrap();
    raw["combined"][0]["collapsed"] = json!(true);
    assert!(parse_state(&raw.to_string()).unwrap().combined[0].collapsed);
    remove(&mut raw["combined"][0], "collapsed");
    assert!(!parse_state(&raw.to_string()).unwrap().combined[0].collapsed);
}

#[test]
fn parses_spend_additions() {
    let state = parse_state(FULL).unwrap();
    let week = state.spend.period(SpendPeriod::Last7Days).unwrap();
    assert_eq!(week.cost_per_mtok_usd_micros, Some(2_000_000));
    assert_eq!(week.by_provider[1].models[1].cost_per_mtok_usd_micros, None);
    assert_eq!(
        week.by_provider[0].cost_per_mtok_usd_micros,
        Some(2_000_000)
    );
    let projects = state.spend.today.projects.as_ref().unwrap();
    assert_eq!(projects[0].project, None);
    assert_eq!(projects[0].share_permille, 1000);
    assert_eq!(projects[0].cost_per_mtok_usd_micros, Some(2_000_000));
    assert_eq!(projects[0].by_provider[1].provider, "codex");
    assert!(state.spend.has_projects());
    assert!(state.spend.has_period(SpendPeriod::Last7Days));
    assert_eq!(
        state.spend.period(SpendPeriod::Last30Days),
        Some(&state.spend.last_30_days)
    );
}

#[test]
fn older_spend_hides_new_choices() {
    let state = edited(|state| {
        remove(&mut state["spend"], "last_7_days");
        for period in ["today", "yesterday", "last_30_days"] {
            remove(&mut state["spend"][period], "projects");
            remove(&mut state["spend"][period], "projects_other");
            remove(&mut state["spend"][period], "cost_per_mtok_usd_micros");
        }
    });
    assert!(!state.spend.has_period(SpendPeriod::Last7Days));
    assert!(!state.spend.has_projects());
    assert_eq!(state.spend.today.cost_per_mtok_usd_micros, None);
}

#[test]
fn parses_projects_other() {
    let other: OtherProjects = serde_json::from_value(json!({
        "count": 4, "cost_usd_micros": 3_300_000, "total_tokens": 9_000_000,
        "partial": false, "share_permille": 266
    }))
    .unwrap();
    assert_eq!(other.count, 4);
    assert_eq!(other.share_permille, 266);
    assert_eq!(other.cost_per_mtok_usd_micros, None);
    let rated: OtherProjects = serde_json::from_value(json!({
        "count": 4, "cost_usd_micros": 3_300_000, "total_tokens": 9_000_000,
        "partial": false, "share_permille": 266, "cost_per_mtok_usd_micros": 366_667
    }))
    .unwrap();
    assert_eq!(rated.cost_per_mtok_usd_micros, Some(366_667));
}

#[test]
fn parses_the_0_6_display_copy() {
    let state = edited(|state| {
        state["display"]["density"] = json!("compact");
        state["display"]["panel_limits"] =
            json!([{"account_id": "claude:main", "window": "session"}]);
        state["display"]["starred_accounts"] = json!(["claude:main"]);
        state["display"]["panel_position"] = json!({"box": "left", "index": 3});
    });
    let display = &state.display;
    assert_eq!(display.density, Density::Compact);
    assert_eq!(display.time_format, TimeFormat::Auto);
    assert_eq!(display.panel_mode, PanelMode::Headline);
    assert_eq!(display.panel_label, PanelLabel::Percent);
    assert_eq!(display.panel_limits[0].window, "session");
    assert!(display.is_starred("claude:main"));
    assert_eq!(display.panel_position.panel_box, PanelBox::Left);
    assert_eq!(display.panel_position.index, 3);
    assert!(display.hide_on_screen_share);
}

#[test]
fn older_daemons_leave_the_0_6_1_fields_empty() {
    let state = edited(|state| {
        let pace = &mut state["accounts"][0]["windows"][0]["pace"];
        remove(pace, "basis");
        remove(pace, "active_left_seconds");
        remove(&mut state["display"], "show_breakdown");
    });
    let pace = &state.accounts[0].windows[0].pace;
    assert_eq!(pace.basis, None);
    assert_eq!(pace.active_left_seconds, None);
    assert!(state.display.show_breakdown);
}

#[test]
fn parses_the_pace_basis_and_the_work_left() {
    let state = edited(|state| {
        let windows = &mut state["accounts"][0]["windows"];
        windows[0]["pace"]["basis"] = json!("paused");
        windows[0]["pace"]["active_left_seconds"] = json!(10_800);
        windows[1]["pace"]["basis"] = json!("tomorrow");
        windows[1]["pace"]["active_left_seconds"] = json!(-4);
    });
    let windows = &state.accounts[0].windows;
    assert_eq!(windows[0].pace.basis, Some(PaceBasis::Paused));
    assert_eq!(windows[0].pace.active_left_seconds, Some(10_800));
    assert_eq!(windows[1].pace.basis, None);
    assert_eq!(windows[1].pace.active_left_seconds, None);
    for (text, basis) in [("recent", PaceBasis::Recent), ("window", PaceBasis::Window)] {
        let state = edited(|state| {
            state["accounts"][0]["windows"][0]["pace"]["basis"] = json!(text);
        });
        assert_eq!(state.accounts[0].windows[0].pace.basis, Some(basis));
    }
}

#[test]
fn parses_the_breakdown_switch_and_other_rates() {
    let state = edited(|state| {
        state["display"]["show_breakdown"] = json!(false);
        state["spend"]["today"]["by_provider"][0]["models_other"] = json!({
            "count": 2,
            "total_tokens": 1_900_000,
            "cost_usd_micros": 1_490_000,
            "partial": false,
            "cost_per_mtok_usd_micros": 784_211
        });
    });
    assert!(!state.display.show_breakdown);
    let other = state.spend.today.by_provider[0]
        .models_other
        .as_ref()
        .unwrap();
    assert_eq!(other.cost_per_mtok_usd_micros, Some(784_211));
}

#[test]
fn cli_login_recovery_carries_the_account() {
    let state = edited(|state| {
        state["accounts"][0]["recovery"] = json!({
            "action": "cli_login",
            "command": "codex login",
            "account_id": "codex:work"
        });
    });
    assert_eq!(
        state.accounts[0].recovery,
        RecoveryField::Offered(Recovery::CliLogin {
            command: "codex login".into(),
            account_id: Some("codex:work".into()),
        })
    );
}
