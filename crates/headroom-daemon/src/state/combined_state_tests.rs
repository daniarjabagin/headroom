use headroom_core::pace::{Severity, Tone};

use super::tests::{assemble_sample, check_snapshot, record, sample_model};
use super::*;
use crate::dismissed::DismissedHome;
use crate::model::{SnapshotEntry, SnapshotOrigin};
use crate::settings::HeadlineMode;
use crate::testing::{CODEX, session, snapshot, ts, weekly};

fn combined_model() -> Model {
    let mut model = sample_model();
    let mut personal = record(CODEX, "personal", 4);
    personal.label = Some("Personal".into());
    let mut limits = snapshot(
        vec![
            session(20.0, "2026-09-23T12:30:00Z"),
            weekly(60.0, "2026-09-25T10:00:00Z"),
        ],
        "2026-09-23T09:59:00Z",
    );
    limits.identity.plan = Some("Plus".into());
    model.snapshots.insert(
        personal.id().clone(),
        SnapshotEntry {
            snapshot: limits,
            origin: SnapshotOrigin::Refreshed,
        },
    );
    model.accounts.push(personal);
    model.settings.display.combine_accounts = true;
    model
}

#[test]
fn combined_state_matches_snapshot() {
    let payload = assemble_sample(&combined_model());
    check_snapshot(
        "state_combined.json",
        include_str!("snapshots/state_combined.json"),
        &payload,
    );
}

#[test]
fn combined_state_groups_the_visible_codex_accounts() {
    let payload = assemble_sample(&combined_model());
    assert!(payload.display.combine_accounts);
    assert_eq!(payload.combined.len(), 1);
    let group = &payload.combined[0];
    assert_eq!(group.account_ids, ["codex:work", "codex:personal"]);
    let shape: Vec<_> = group
        .windows
        .iter()
        .map(|w| (w.id.as_str(), w.capacity_percent))
        .collect();
    assert_eq!(shape, [("session", 200), ("weekly", 100)]);
    let session = &group.windows[0];
    assert_eq!(session.resets_at, Some(ts("2026-09-23T12:00:00Z")));
    assert_eq!(session.pace.severity, Severity::Healthy);
    assert_eq!(session.tone, Tone::Good);
    assert_eq!(payload.accounts.len(), 4);
    let headline = payload.headline.unwrap();
    assert_eq!(headline.account_id, "claude:main");
    assert_eq!(headline.tone, Tone::Critical);
    assert!(!headline.combined);
}

#[test]
fn the_combined_list_is_empty_when_the_setting_is_off() {
    let mut model = combined_model();
    model.settings.display.combine_accounts = false;
    let payload = assemble_sample(&model);
    assert!(payload.combined.is_empty());
    let json = serde_json::to_value(&payload).unwrap();
    assert_eq!(json["combined"], serde_json::json!([]));
    assert_eq!(
        json["display"]["combine_accounts"],
        serde_json::json!(false)
    );
    assert!(!payload.headline.unwrap().combined);
}

#[test]
fn a_dismissed_account_leaves_its_group() {
    let mut model = combined_model();
    let personal = model.accounts.last().unwrap().reference.clone();
    model.dismissed.insert(DismissedHome::of(&personal));
    assert!(assemble_sample(&model).combined.is_empty());
}

#[test]
fn a_pin_on_a_grouped_account_shows_the_combined_window() {
    let mut model = combined_model();
    model.settings.headline = HeadlineMode::Pinned {
        account_id: "codex:personal".into(),
        window: "weekly".into(),
    };
    let headline = assemble_sample(&model).headline.unwrap();
    assert!(headline.combined);
    assert_eq!(headline.window, "weekly");
    assert_eq!(headline.account_count, 1);
    assert_eq!(headline.account_id, "codex:personal");
    assert_eq!(headline.account_label, None);
    assert!((headline.remaining_percent - 40.0).abs() < f64::EPSILON);
}

#[test]
fn combined_payload_parses_back_from_json() {
    let payload = assemble_sample(&combined_model());
    let json = serde_json::to_string(&payload).unwrap();
    let parsed: StatePayload = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.combined, payload.combined);
    assert_eq!(parsed.headline, payload.headline);
}
