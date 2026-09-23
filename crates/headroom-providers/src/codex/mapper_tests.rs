use headroom_core::quota::WindowId;

use super::super::test_support::at;
use super::*;

const FULL: &str = include_str!("fixtures/usage_full.json");
const WEEKLY_ONLY: &str = include_str!("fixtures/usage_weekly_only.json");
const NOW: &str = "2026-09-21T14:13:20Z";

fn identity() -> AccountIdentity {
    AccountIdentity {
        email: Some("someone@example.com".into()),
        plan: Some("Free".into()),
        stable_key: "user/acct".into(),
    }
}

fn map(json: &str) -> LimitsSnapshot {
    let response: UsageResponse = serde_json::from_str(json).unwrap();
    map_usage(&response, identity(), at(NOW))
}

fn count(id: &str, label: &str, value: u64, unit: &str) -> Balance {
    Balance {
        id: id.into(),
        label: label.into(),
        amount: BalanceAmount::Count {
            value,
            unit: unit.into(),
        },
    }
}

#[test]
fn full_response_maps_every_window_and_balance() {
    let snapshot = map(FULL);
    let summary: Vec<_> = snapshot
        .windows
        .iter()
        .map(|window| (window.id.clone(), window.label.as_str(), window.used))
        .collect();
    assert_eq!(
        summary,
        [
            (WindowId::Session, "Session", Percent::new(37.5)),
            (WindowId::Weekly, "Weekly", Percent::new(62.0)),
            (
                WindowId::Model("GPT-5.3-Codex-Spark".into()),
                "Spark",
                Percent::new(4.0)
            ),
            (
                WindowId::Model("GPT-5.3-Codex-Spark:weekly".into()),
                "Spark Weekly",
                Percent::new(1.25)
            ),
        ]
    );
    assert_eq!(
        snapshot.balances,
        [
            count("credits", "Credits", 821, "credits"),
            count("resets", "Resets", 3, "resets"),
        ]
    );
    assert_eq!(snapshot.identity.plan.as_deref(), Some("Plus"));
    assert_eq!(snapshot.source, LimitsSource::Live);
    assert_eq!(snapshot.fetched_at, at(NOW));
}

#[test]
fn absolute_reset_wins_and_relative_reset_is_added_to_now() {
    let snapshot = map(FULL);
    assert_eq!(
        snapshot.windows[0].resets_at,
        Some(at("2026-09-21T15:13:20Z"))
    );
    assert_eq!(
        snapshot.windows[1].resets_at,
        Some(at("2026-09-25T14:13:20Z"))
    );
    assert_eq!(
        snapshot.windows[2].resets_at,
        Some(at("2026-09-21T16:13:20.5Z"))
    );
    assert_eq!(snapshot.windows[1].period, Some(WindowId::WEEKLY_PERIOD));
}

#[test]
fn weekly_window_in_primary_slot_is_weekly() {
    let snapshot = map(WEEKLY_ONLY);
    assert_eq!(snapshot.windows.len(), 1);
    assert_eq!(snapshot.windows[0].id, WindowId::Weekly);
    assert_eq!(snapshot.windows[0].used, Percent::new(28.0));
    assert_eq!(
        snapshot.windows[0].resets_at,
        Some(at("2026-09-22T15:13:20Z"))
    );
    assert_eq!(snapshot.identity.plan.as_deref(), Some("Pro 5x"));
    assert!(snapshot.balances.is_empty());
}

#[test]
fn credits_show_only_when_held_unlimited_or_positive() {
    let cases = [
        (
            r#"{"has_credits":false,"unlimited":false,"balance":"0"}"#,
            None,
        ),
        (r#"{"balance":0}"#, None),
        (
            r#"{"has_credits":true,"unlimited":false,"balance":"0"}"#,
            Some(0),
        ),
        (
            r#"{"has_credits":false,"unlimited":true,"balance":"0"}"#,
            Some(0),
        ),
        (
            r#"{"has_credits":false,"unlimited":false,"balance":"5.5"}"#,
            Some(5),
        ),
        (r#"{"has_credits":true,"unlimited":false}"#, None),
    ];
    for (credits, expected) in cases {
        let snapshot = map(&format!(r#"{{"credits":{credits}}}"#));
        let shown = expected.map(|value| count("credits", "Credits", value, "credits"));
        assert_eq!(snapshot.balances, Vec::from_iter(shown), "{credits}");
    }
}

#[test]
fn resets_show_only_when_available() {
    let none = map(r#"{"rate_limit_reset_credits":{"available_count":0}}"#);
    assert!(none.balances.is_empty());
    let one = map(r#"{"rate_limit_reset_credits":{"available_count":"1"}}"#);
    assert_eq!(one.balances, [count("resets", "Resets", 1, "resets")]);
}

#[test]
fn empty_object_keeps_claim_plan_and_has_no_windows() {
    let snapshot = map("{}");
    assert!(snapshot.windows.is_empty());
    assert!(snapshot.balances.is_empty());
    assert_eq!(snapshot.identity, identity());
}

#[test]
fn window_without_used_percent_is_skipped() {
    let snapshot = map(
        r#"{"rate_limit":{"primary_window":{"limit_window_seconds":18000},
            "secondary_window":{"used_percent":"n/a","limit_window_seconds":604800}}}"#,
    );
    assert!(snapshot.windows.is_empty());
}

#[test]
fn additional_limit_falls_back_to_metered_feature_name() {
    let snapshot = map(r#"{"additional_rate_limits":[
            {"limit_name":null,"metered_feature":"codex_spark_preview",
             "rate_limit":{"primary_window":{"used_percent":9,"limit_window_seconds":86400}}},
            {"limit_name":"Unnamed","rate_limit":null}]}"#);
    assert_eq!(snapshot.windows.len(), 1);
    assert_eq!(
        snapshot.windows[0].id,
        WindowId::Model("codex_spark_preview:1d".into())
    );
    assert_eq!(snapshot.windows[0].label, "Spark Preview 1d");
}
