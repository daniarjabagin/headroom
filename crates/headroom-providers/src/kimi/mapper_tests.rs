use serde_json::json;

use super::*;

fn raw(text: &str) -> RawUsages {
    serde_json::from_str(text).unwrap()
}

fn summary(mapped: &MappedUsage) -> Vec<(WindowId, String, f64, Option<i64>)> {
    mapped
        .windows
        .iter()
        .map(|w| {
            (
                w.id.clone(),
                w.label.clone(),
                w.used.value(),
                w.period.map(|p| p.as_secs()),
            )
        })
        .collect()
}

fn at(text: &str) -> Option<Timestamp> {
    text.parse().ok()
}

#[test]
fn string_counts_map_exactly_to_session_and_weekly() {
    let mapped = map_usage(&raw(include_str!("fixtures/usages_counts.json"))).unwrap();
    assert_eq!(mapped.plan.as_deref(), Some("Basic"));
    assert_eq!(
        summary(&mapped),
        [
            (WindowId::Session, "Session".into(), 20.0, Some(18_000)),
            (WindowId::Weekly, "Weekly".into(), 25.0, Some(604_800)),
        ]
    );
    assert_eq!(
        mapped.windows[0].resets_at,
        at("2026-09-23T12:17:43.13902Z")
    );
    assert_eq!(
        mapped.windows[1].resets_at,
        at("2026-09-27T02:17:43.13902Z")
    );
}

#[test]
fn missing_used_is_derived_from_remaining_and_counts_beat_ratios() {
    let mapped = map_usage(&raw(include_str!("fixtures/usages_remaining_only.json"))).unwrap();
    assert_eq!(mapped.plan, None);
    assert_eq!(
        summary(&mapped),
        [
            (WindowId::Session, "Session".into(), 0.0, Some(18_000)),
            (WindowId::Weekly, "Weekly".into(), 0.0, Some(604_800)),
        ]
    );
    assert_eq!(
        mapped.windows[1].resets_at,
        at("2026-09-26T12:32:15.37698Z")
    );
}

#[test]
fn ratio_pools_add_monthly_windows_without_a_weekly_summary() {
    let mapped = map_usage(&raw(include_str!("fixtures/usages_ratio_pools.json"))).unwrap();
    assert_eq!(
        summary(&mapped),
        [
            (WindowId::Session, "Session".into(), 25.0, Some(18_000)),
            (
                WindowId::Other("monthly_code".into()),
                "Monthly (Code)".into(),
                0.0,
                None
            ),
            (
                WindowId::Other("monthly".into()),
                "Monthly".into(),
                0.0056 * 100.0,
                None
            ),
        ]
    );
    assert_eq!(mapped.windows[2].resets_at, at("2026-10-17T00:00:00Z"));
}

#[test]
fn wrapped_camel_case_pools_are_read() {
    let mapped = map_usage(&raw(include_str!("fixtures/usages_wrapped.json"))).unwrap();
    let used: Vec<_> = mapped.windows.iter().map(|w| w.used.value()).collect();
    assert_eq!(used, [0.1719 * 100.0, 0.346 * 100.0]);
    assert_eq!(mapped.windows[0].id, WindowId::Session);
    assert_eq!(mapped.windows[1].resets_at, at("2026-09-28T23:07:12Z"));
}

#[test]
fn used_above_limit_is_kept_and_remaining_above_limit_is_zero() {
    let body = json!({
        "usage": {"limit": "40", "used": "50"},
        "limits": [{"window": {"duration": 5, "timeUnit": "TIME_UNIT_HOUR"},
                    "detail": {"limit": "100", "remaining": "130"}}]
    });
    let mapped = map_usage(&serde_json::from_value(body).unwrap()).unwrap();
    let used: Vec<_> = mapped.windows.iter().map(|w| w.used.value()).collect();
    assert_eq!(used, [0.0, 125.0]);
}

#[test]
fn other_window_lengths_get_their_own_ids_after_session_and_weekly() {
    let body = json!({
        "limits": [
            {"window": {"duration": 1, "timeUnit": "TIME_UNIT_HOUR"},
             "detail": {"limit": "8", "used": "2"}},
            {"window": {"duration": 7, "timeUnit": "TIME_UNIT_DAY"},
             "detail": {"limit": 1000, "used": 10}},
            {"window": {"duration": 90, "timeUnit": "TIME_UNIT_SECOND"},
             "detail": {"limit": "3", "used": "3"}}
        ]
    });
    let mapped = map_usage(&serde_json::from_value(body).unwrap()).unwrap();
    assert_eq!(
        summary(&mapped),
        [
            (WindowId::Weekly, "Weekly".into(), 1.0, Some(604_800)),
            (
                WindowId::Other("3600s".into()),
                "1-hour".into(),
                25.0,
                Some(3_600)
            ),
            (
                WindowId::Other("90s".into()),
                "90-second".into(),
                100.0,
                Some(90)
            ),
        ]
    );
}

#[test]
fn a_malformed_section_keeps_the_others() {
    let mut body: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/usages_counts.json")).unwrap();
    body["usage"]["used"] = json!("not-a-number");
    body["usage"]["limit"] = json!("0");
    let mapped = map_usage(&serde_json::from_value(body).unwrap()).unwrap();
    let ids: Vec<_> = mapped.windows.iter().map(|w| w.id.clone()).collect();
    assert_eq!(ids, [WindowId::Session]);
}

#[test]
fn fractional_or_negative_counts_are_not_guessed() {
    let body = json!({
        "usage": {"limit": "100", "used": "2.5"},
        "limits": [{"window": {"duration": 300, "timeUnit": "TIME_UNIT_MINUTE"},
                    "detail": {"limit": "100", "remaining": "-1"}},
                   {"window": {"duration": 300, "timeUnit": "TIME_UNIT_FORTNIGHT"},
                    "detail": {"limit": "100", "used": "1"}}]
    });
    let error = map_usage(&serde_json::from_value(body).unwrap()).unwrap_err();
    assert!(
        matches!(error, ProviderError::InvalidResponse(_)),
        "{error:?}"
    );
}

#[test]
fn a_response_without_quotas_is_invalid() {
    for body in [
        json!({}),
        json!({"limits": [], "usages": {}}),
        json!({"code": 0, "data": {}}),
    ] {
        assert!(map_usage(&serde_json::from_value(body).unwrap()).is_err());
    }
}

#[test]
fn plan_levels_are_title_cased() {
    let plan = |level: &str| {
        let body =
            json!({"user": {"membership": {"level": level}}, "usage": {"limit": "1", "used": "0"}});
        map_usage(&serde_json::from_value(body).unwrap())
            .unwrap()
            .plan
    };
    assert_eq!(plan("LEVEL_YEARLY_PRO").as_deref(), Some("Yearly Pro"));
    assert_eq!(plan("allegretto").as_deref(), Some("Allegretto"));
    assert_eq!(plan("LEVEL_"), None);
}
