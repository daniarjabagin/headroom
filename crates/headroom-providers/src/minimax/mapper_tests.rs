use serde_json::json;

use super::*;

fn raw(text: &str) -> RawRemains {
    serde_json::from_str(text).unwrap()
}

fn summary(windows: &[QuotaWindow]) -> Vec<(WindowId, String, f64, Option<i64>)> {
    windows
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
fn remaining_percentages_become_used_for_the_general_model() {
    let windows = map_remains(&raw(include_str!("fixtures/remains_ok.json"))).unwrap();
    assert_eq!(
        summary(&windows),
        [
            (WindowId::Session, "Session".into(), 38.0, Some(18_000)),
            (WindowId::Weekly, "Weekly".into(), 15.0, Some(604_800)),
        ]
    );
    assert_eq!(windows[0].resets_at, at("2026-09-23T13:00:00Z"));
    assert_eq!(windows[1].resets_at, at("2026-09-28T00:00:00Z"));
}

#[test]
fn unlimited_windows_read_as_unused() {
    let windows =
        map_remains(&raw(include_str!("fixtures/remains_unlimited_weekly.json"))).unwrap();
    assert_eq!(
        summary(&windows),
        [
            (WindowId::Session, "Session".into(), 100.0, Some(18_000)),
            (
                WindowId::Weekly,
                "Weekly (Unlimited)".into(),
                0.0,
                Some(604_800)
            ),
        ]
    );
}

#[test]
fn a_weekly_boost_scales_the_allowance_and_string_numbers_parse() {
    let windows = map_remains(&raw(include_str!("fixtures/remains_boost.json"))).unwrap();
    assert_eq!(
        summary(&windows),
        [
            (WindowId::Session, "Session".into(), 62.5, Some(18_000)),
            (WindowId::Weekly, "Weekly".into(), 20.0, Some(604_800)),
        ]
    );
    assert_eq!(windows[0].resets_at, at("2026-09-23T13:00:00Z"));
}

#[test]
fn remaining_above_the_allowance_reads_as_unused() {
    let body = json!({
        "model_remains": [{"model_name": "general",
            "current_interval_remaining_percent": 130,
            "current_weekly_remaining_percent": 160, "weekly_boost_permille": 1500}],
        "base_resp": {"status_code": 0}
    });
    let windows = map_remains(&serde_json::from_value(body).unwrap()).unwrap();
    let used: Vec<_> = windows.iter().map(|w| w.used.value()).collect();
    assert_eq!(used, [0.0, 0.0]);
    assert!(windows.iter().all(|w| w.resets_at.is_none()));
    assert_eq!(windows[0].period, Some(WindowId::SESSION_PERIOD));
    assert_eq!(windows[1].period, Some(WindowId::WEEKLY_PERIOD));
}

#[test]
fn a_broken_window_keeps_the_other() {
    let body = json!({
        "model_remains": [{"model_name": "general",
            "current_interval_remaining_percent": -5,
            "current_weekly_remaining_percent": "40"}]
    });
    let windows = map_remains(&serde_json::from_value(body).unwrap()).unwrap();
    assert_eq!(
        summary(&windows),
        [(WindowId::Weekly, "Weekly".into(), 60.0, Some(604_800))]
    );
}

#[test]
fn a_missing_general_model_or_windows_is_invalid() {
    for body in [
        json!({"model_remains": [{"model_name": "video", "current_interval_remaining_percent": 90}],
               "base_resp": {"status_code": 0}}),
        json!({"model_remains": [{"model_name": "general"}]}),
        json!({"base_resp": {"status_code": 0}}),
    ] {
        let error = map_remains(&serde_json::from_value(body).unwrap()).unwrap_err();
        assert!(
            matches!(error, ProviderError::InvalidResponse(_)),
            "{error:?}"
        );
    }
}

#[test]
fn status_codes_map_to_typed_errors() {
    let error = |text: &str| map_remains(&raw(text)).unwrap_err();
    assert_eq!(
        error(include_str!("fixtures/no_plan.json")),
        ProviderError::NoSubscription {
            detail: NO_PLAN.into()
        }
    );
    assert_eq!(
        error(include_str!("fixtures/invalid_key.json")),
        ProviderError::SignInExpired
    );
    assert_eq!(
        error(r#"{"base_resp":{"status_code":2049,"status_msg":"invalid api key"}}"#),
        ProviderError::SignInExpired
    );
    assert_eq!(
        error(r#"{"base_resp":{"status_code":1002,"status_msg":"rate limit exceeded"}}"#),
        ProviderError::rate_limited(None)
    );
    assert_eq!(
        error(r#"{"base_resp":{"status_code":1013,"status_msg":"internal error"}}"#),
        ProviderError::InvalidResponse("MiniMax returned status 1013: internal error".into())
    );
}
