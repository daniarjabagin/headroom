use jiff::tz::TimeZone;
use serde_json::json;

use super::*;
use crate::payload::Severity;

fn at(text: &str) -> Timestamp {
    text.parse().unwrap()
}

fn group() -> CombinedGroup {
    serde_json::from_value(json!({
        "provider": "codex",
        "provider_name": "Codex",
        "account_ids": ["codex:work", "codex:personal"],
        "accounts": [
            {"account_id": "codex:work", "label": "Work", "plan": "Pro"},
            {"account_id": "codex:personal", "label": "Personal", "plan": "Plus"}
        ],
        "windows": [{
            "id": "session", "label": "Session", "capacity_percent": 200,
            "remaining_percent": 125.0, "used_percent": 75.0,
            "resets_at": "2026-09-23T12:00:00Z", "tone": "good",
            "pace": {"severity": "healthy", "even_pace_percent": 110.0,
                     "projected_percent": 131.67, "spare_percent": 68.33, "runs_out_at": null},
            "segments": [
                {"account_id": "codex:work", "label": "Work", "remaining_percent": 45.0,
                 "used_percent": 55.0, "resets_at": "2026-09-23T12:00:00Z", "tone": "warning"},
                {"account_id": "codex:personal", "label": null, "remaining_percent": 80.0,
                 "used_percent": 20.0, "resets_at": "2026-09-23T12:30:00Z", "tone": "good"}
            ]
        }]
    }))
    .unwrap()
}

fn row(display: &Display) -> CombinedRow {
    let locale = Locale::new(Lang::En, TimeZone::UTC);
    combined_row(
        &locale,
        &group().windows[0],
        &[],
        display,
        at("2026-09-23T10:00:00Z"),
    )
}

#[test]
fn titles_and_plans() {
    assert_eq!(group_title(Lang::En, &group()), "Codex · 2 accounts");
    assert_eq!(group_title(Lang::Ru, &group()), "Codex · 2 аккаунта");
    assert_eq!(group_plans(&group()).as_deref(), Some("Pro · Plus"));
}

#[test]
fn row_reads_on_the_capacity_scale() {
    let left = row(&Display::default());
    assert_eq!(left.label, "Session");
    assert_eq!(left.headline, "125% left of 200%");
    assert_eq!(left.trailing, "Resets in 2h 0m");
    assert_eq!(
        left.forecast.as_deref(),
        Some("At this pace: ~68% left at reset")
    );
    assert_eq!(left.segments.len(), 2);
    assert!((left.segments[0].fraction - 0.45).abs() < 1e-9);
    assert_eq!(left.segments[0].tone, Tone::Warning);
    let used = row(&Display {
        value_mode: ValueMode::Used,
        ..Display::default()
    });
    assert_eq!(used.headline, "75% used of 200%");
    assert!((used.segments[1].fraction - 0.2).abs() < 1e-9);
}

#[test]
fn breakdown_names_every_account() {
    let tip = row(&Display::default()).breakdown;
    let lines: Vec<&str> = tip.lines().collect();
    assert_eq!(lines[0], "Work 45% · codex:personal 80%");
    assert_eq!(lines[1], "Work · Resets in 2h 0m");
    assert_eq!(lines[2], "codex:personal · Resets in 2h 30m");
}

#[test]
fn running_out_gets_a_flame_note() {
    let mut combined = group();
    combined.windows[0].pace.severity = Severity::RunningOut;
    let locale = Locale::new(Lang::En, TimeZone::UTC);
    let quiet = Display {
        show_forecast: false,
        ..Display::default()
    };
    let result = combined_row(
        &locale,
        &combined.windows[0],
        &[],
        &quiet,
        at("2026-09-23T10:00:00Z"),
    );
    assert!(result.note.unwrap().flame);
}

fn member(id: &str, even_pace: Option<f64>) -> Account {
    serde_json::from_value(json!({
        "id": id, "provider": "codex", "provider_name": "Codex", "label": null,
        "email": null, "plan": null, "hidden": false, "status": "fresh", "error": null,
        "updated_at": null, "balances": [], "notices": [], "usage_home": "~/.codex",
        "windows": [{"id": "session", "label": "Session", "used_percent": 55.0,
            "remaining_percent": 45.0, "resets_at": null, "tone": "good", "hidden": false,
            "pace": {"severity": "healthy", "even_pace_percent": even_pace,
                     "projected_percent": null, "spare_percent": null, "runs_out_at": null}}]
    }))
    .unwrap()
}

#[test]
fn segments_carry_the_even_pace_tick_of_their_account() {
    let locale = Locale::new(Lang::En, TimeZone::UTC);
    let members = [
        member("codex:work", Some(40.0)),
        member("codex:personal", None),
    ];
    let window = &group().windows[0];
    let now = at("2026-09-23T10:00:00Z");
    let left = combined_row(&locale, window, &members, &Display::default(), now);
    assert!((left.segments[0].tick.unwrap() - 0.6).abs() < 1e-9);
    assert_eq!(left.segments[1].tick, None);
    let used = Display {
        value_mode: ValueMode::Used,
        ..Display::default()
    };
    let used_row = combined_row(&locale, window, &members, &used, now);
    assert!((used_row.segments[0].tick.unwrap() - 0.4).abs() < 1e-9);
    assert_eq!(
        combined_row(&locale, window, &[], &used, now).segments[0].tick,
        None
    );
}
