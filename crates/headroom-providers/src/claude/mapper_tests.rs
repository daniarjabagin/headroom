use serde_json::json;

use super::*;

const FULL: &str = include_str!("fixtures/usage_full.json");
const MINIMAL: &str = include_str!("fixtures/usage_minimal.json");
const EXTRA_DISABLED: &str = include_str!("fixtures/usage_extra_disabled.json");

fn map(text: &str) -> MappedUsage {
    map_usage(&serde_json::from_str(text).unwrap())
}

fn at(text: &str) -> Timestamp {
    text.parse().unwrap()
}

fn summary(windows: &[QuotaWindow]) -> Vec<(WindowId, &str, f64, Option<i64>)> {
    windows
        .iter()
        .map(|w| {
            (
                w.id.clone(),
                w.label.as_str(),
                w.used.value(),
                w.period.map(|p| p.as_secs()),
            )
        })
        .collect()
}

fn usd(id: &str, label: &str, micros: i64) -> Balance {
    Balance {
        id: id.to_owned(),
        label: label.to_owned(),
        amount: BalanceAmount::Usd(MicroUsd(micros)),
    }
}

#[test]
fn full_response_maps_every_window_once() {
    let mapped = map(FULL);
    assert_eq!(
        summary(&mapped.windows),
        [
            (WindowId::Session, "Session", 37.0, Some(18_000)),
            (WindowId::Weekly, "Weekly", 51.5, Some(604_800)),
            (WindowId::Model("opus".into()), "Opus", 12.0, Some(604_800)),
            (
                WindowId::Model("fable".into()),
                "Fable",
                99.0,
                Some(604_800)
            ),
        ]
    );
    assert_eq!(
        mapped.windows[0].resets_at,
        Some(at("2026-09-23T13:59:59.886864Z"))
    );
    assert_eq!(
        mapped.windows[3].resets_at,
        Some(at("2026-09-26T14:59:59.887158Z"))
    );
}

#[test]
fn full_response_maps_extra_usage_to_usd_balances() {
    assert_eq!(
        map(FULL).balances,
        [
            usd("extra_usage_spent", "Extra usage spent", 12_500_000),
            usd("extra_usage_limit", "Extra usage limit", 50_000_000),
        ]
    );
}

#[test]
fn minimal_response_maps_windows_without_resets() {
    let mapped = map(MINIMAL);
    assert_eq!(
        summary(&mapped.windows),
        [
            (WindowId::Session, "Session", 42.5, Some(18_000)),
            (WindowId::Weekly, "Weekly", 10.0, Some(604_800)),
            (
                WindowId::Model("sonnet".into()),
                "Sonnet",
                25.0,
                Some(604_800)
            ),
            (WindowId::Model("fable".into()), "Fable", 33.0, None),
        ]
    );
    assert_eq!(mapped.windows[1].resets_at, None);
    assert_eq!(mapped.balances.len(), 2);
}

#[test]
fn null_windows_and_disabled_extra_usage_are_skipped() {
    let mapped = map(EXTRA_DISABLED);
    assert_eq!(
        summary(&mapped.windows),
        [(WindowId::Weekly, "Weekly", 0.0, Some(604_800))]
    );
    assert!(mapped.balances.is_empty());
}

#[test]
fn empty_object_maps_to_nothing() {
    let mapped = map("{}");
    assert!(mapped.windows.is_empty());
    assert!(mapped.balances.is_empty());
}

#[test]
fn unknown_limit_kinds_become_other_windows() {
    let raw = json!({
        "limits": [
            { "kind": "daily_surface", "group": "daily", "percent": 5 },
            { "kind": "weekly_scoped", "group": "weekly", "percent": 7,
              "scope": { "model": { "display_name": "Claude Opus" } } },
            { "kind": "no_percent" },
            "garbage",
            { "percent": 3 }
        ]
    });
    let mapped = map(&raw.to_string());
    assert_eq!(
        summary(&mapped.windows),
        [
            (
                WindowId::Other("daily_surface".into()),
                "Daily Surface",
                5.0,
                None
            ),
            (
                WindowId::Model("claude_opus".into()),
                "Claude Opus",
                7.0,
                Some(604_800)
            ),
        ]
    );
}

#[test]
fn malformed_scoped_windows_are_skipped() {
    let raw = json!({
        "seven_day_sonnet": { "utilization": "high" },
        "seven_day_oauth_apps": { "utilization": 4 },
        "seven_day_": { "utilization": 1 }
    });
    assert_eq!(
        summary(&map(&raw.to_string()).windows),
        [(
            WindowId::Model("oauth_apps".into()),
            "Oauth Apps",
            4.0,
            Some(604_800)
        )]
    );
}

#[test]
fn reset_times_accept_naive_and_epoch_values() {
    assert_eq!(
        reset_time(&json!("2026-09-23T10:00:00.5")),
        Some(at("2026-09-23T10:00:00.5Z"))
    );
    assert_eq!(
        reset_time(&json!(1_790_000_000)),
        Some(Timestamp::from_second(1_790_000_000).unwrap())
    );
    assert_eq!(
        reset_time(&json!(1_790_000_000_123_i64)),
        Some(Timestamp::from_millisecond(1_790_000_000_123).unwrap())
    );
    assert_eq!(reset_time(&json!("soon")), None);
    assert_eq!(reset_time(&json!(true)), None);
}

#[test]
fn extra_usage_respects_currency_and_decimal_places() {
    let euro =
        json!({ "extra_usage": { "is_enabled": true, "used_credits": 100, "currency": "EUR" } });
    assert!(map(&euro.to_string()).balances.is_empty());
    let three_digits = json!({
        "extra_usage": { "is_enabled": true, "used_credits": 1500, "monthly_limit": 0, "decimal_places": 3 }
    });
    assert_eq!(
        map(&three_digits.to_string()).balances,
        [usd("extra_usage_spent", "Extra usage spent", 1_500_000)]
    );
}
