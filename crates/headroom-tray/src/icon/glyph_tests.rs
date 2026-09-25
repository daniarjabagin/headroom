use serde_json::{Value, json};

use super::*;
use crate::payload::parse_state;

const FULL: &str = include_str!("../../../headroom-daemon/src/state/snapshots/state_full.json");

fn item(provider: &str, value: f64, tone: &str, even: Option<f64>) -> Value {
    json!({
        "account_id": format!("{provider}:1"), "provider": provider,
        "provider_name": provider, "window": "weekly", "window_label": "Weekly",
        "used_percent": 100.0 - value, "remaining_percent": value, "value_percent": value,
        "tone": tone, "even_pace_percent": even, "logo": provider
    })
}

fn state(display: &Value, items: Vec<Value>, tone: Option<&str>) -> State {
    let mut raw: Value = serde_json::from_str(FULL).unwrap();
    for (key, value) in display.as_object().unwrap() {
        raw["display"][key] = value.clone();
    }
    raw["panel_items"] = Value::Array(items);
    raw["panel_tone"] = json!(tone);
    parse_state(&raw.to_string()).unwrap()
}

fn gauge(percent: u8, tick: Option<u8>, tone: Tone) -> Gauge {
    Gauge {
        percent,
        tick,
        tone,
    }
}

#[test]
fn no_state_or_no_items_show_the_themed_mark() {
    assert!(Glyph::from_state(None).is_themed_mark());
    let empty = state(&json!({}), vec![], Some("good"));
    assert!(Glyph::from_state(Some(&empty)).is_themed_mark());
}

#[test]
fn headline_draws_one_ring_from_the_item() {
    let full = parse_state(FULL).unwrap();
    assert_eq!(
        Glyph::from_state(Some(&full)),
        Glyph::Rings(vec![gauge(8, Some(10), Tone::Critical)])
    );
}

#[test]
fn several_keeps_the_two_worst_in_daemon_order() {
    let items = vec![
        item("claude", 72.0, "good", Some(41.5)),
        item("codex", 40.0, "warning", None),
        item("grok", 9.0, "critical", None),
    ];
    let several = state(&json!({"panel_mode": "several"}), items, Some("critical"));
    assert_eq!(
        Glyph::from_state(Some(&several)),
        Glyph::Rings(vec![
            gauge(40, None, Tone::Warning),
            gauge(9, None, Tone::Critical)
        ])
    );
}

#[test]
fn ties_keep_the_first_items() {
    let items = vec![
        item("a", 10.0, "good", None),
        item("b", 20.0, "good", None),
        item("c", 30.0, "good", None),
    ];
    let several = state(&json!({"panel_mode": "several"}), items, None);
    let Glyph::Rings(gauges) = Glyph::from_state(Some(&several)) else {
        panic!("rings expected");
    };
    assert_eq!(
        gauges.iter().map(|g| g.percent).collect::<Vec<_>>(),
        [10, 20]
    );
}

#[test]
fn bar_ticks_follow_the_value_mode() {
    let items = vec![item("claude", 72.0, "good", Some(41.5))];
    let left = state(&json!({"panel_indicator": "bar"}), items.clone(), None);
    assert_eq!(
        Glyph::from_state(Some(&left)),
        Glyph::Bars(vec![gauge(72, Some(59), Tone::Good)])
    );
    let used = state(
        &json!({"panel_indicator": "bar", "value_mode": "used"}),
        items,
        None,
    );
    let Glyph::Bars(gauges) = Glyph::from_state(Some(&used)) else {
        panic!("bars expected");
    };
    assert_eq!(gauges[0].tick, Some(42));
}

#[test]
fn icon_mode_tints_the_mark_only_when_alarming() {
    let items = vec![item("claude", 72.0, "good", None)];
    let warn = state(
        &json!({"panel_mode": "icon"}),
        items.clone(),
        Some("warning"),
    );
    assert_eq!(
        Glyph::from_state(Some(&warn)),
        Glyph::Mark(Some(Tone::Warning))
    );
    let good = state(&json!({"panel_mode": "icon"}), items, Some("good"));
    assert!(Glyph::from_state(Some(&good)).is_themed_mark());
}

#[test]
fn indicator_none_falls_back_to_rings_and_none_none_to_the_mark() {
    let items = vec![item("claude", 72.0, "good", None)];
    let label_only = state(&json!({"panel_indicator": "none"}), items.clone(), None);
    assert!(matches!(
        Glyph::from_state(Some(&label_only)),
        Glyph::Rings(_)
    ));
    let nothing = state(
        &json!({"panel_indicator": "none", "panel_label": "none"}),
        items,
        Some("critical"),
    );
    assert_eq!(
        Glyph::from_state(Some(&nothing)),
        Glyph::Mark(Some(Tone::Critical))
    );
}

#[test]
fn critical_detection() {
    assert!(Glyph::Mark(Some(Tone::Critical)).has_critical());
    assert!(!Glyph::Mark(Some(Tone::Warning)).has_critical());
    assert!(Glyph::Bars(vec![gauge(1, None, Tone::Critical)]).has_critical());
    assert!(!Glyph::Rings(vec![gauge(1, None, Tone::Good)]).has_critical());
}
