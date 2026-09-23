use serde_json::json;

use super::*;
use crate::cursor::client::parse;

const PRO: &str = include_str!("fixtures/period_usage_pro.json");
const TEAM: &str = include_str!("fixtures/period_usage_team.json");
const FREE: &str = include_str!("fixtures/period_usage_free.json");
const REQUEST_BASED: &str = include_str!("fixtures/period_usage_request_based.json");

fn raw(text: &str) -> RawPeriodUsage {
    parse(text.as_bytes()).unwrap()
}

fn mapped(text: &str, plan: Option<&str>) -> MappedUsage {
    match map_period_usage(&raw(text), plan).unwrap() {
        PlanUsage::Mapped(usage) => usage,
        PlanUsage::RequestBased => panic!("expected plan usage"),
    }
}

fn cycle() -> Cycle {
    Cycle {
        resets_at: Some("2026-10-05T00:00:00Z".parse().unwrap()),
        period: Some(SignedDuration::from_hours(30 * 24)),
    }
}

#[test]
fn pro_plan_maps_total_auto_and_api_windows() {
    let usage = mapped(PRO, Some("pro"));
    assert_eq!(
        usage.windows,
        [
            window("total", "Total", Percent::new(36.75), cycle()),
            window("auto", "Auto", Percent::new(12.5), cycle()),
            window("api", "API", Percent::new(60.25), cycle()),
        ]
    );
}

#[test]
fn on_demand_spend_and_limit_become_usd_balances() {
    let usage = mapped(PRO, Some("pro"));
    assert_eq!(
        usage.balances,
        [
            usd_balance("on_demand_spent", "On-demand spent", MicroUsd(12_340_000)),
            usd_balance("on_demand_limit", "On-demand limit", MicroUsd(50_000_000)),
        ]
    );
}

#[test]
fn team_total_comes_from_spend_over_limit() {
    let usage = mapped(TEAM, None);
    assert_eq!(usage.windows.len(), 1);
    assert_eq!(usage.windows[0].used, Percent::new(25.0));
    assert_eq!(usage.windows[0].period, cycle().period);
    assert_eq!(
        usage.balances,
        [
            usd_balance("on_demand_spent", "On-demand spent", MicroUsd(250_000_000)),
            usd_balance(
                "on_demand_limit",
                "On-demand limit",
                MicroUsd(1_000_000_000)
            ),
        ]
    );
}

#[test]
fn team_by_plan_name_also_uses_spend() {
    let mut raw = raw(TEAM);
    raw.spend_limit_usage = None;
    let PlanUsage::Mapped(usage) = map_period_usage(&raw, Some(" Team ")).unwrap() else {
        panic!("expected plan usage");
    };
    assert_eq!(usage.windows[0].used, Percent::new(25.0));
    let PlanUsage::Mapped(usage) = map_period_usage(&raw, Some("pro")).unwrap() else {
        panic!("expected plan usage");
    };
    assert_eq!(usage.windows[0].used, Percent::new(12.0));
}

#[test]
fn disabled_usage_is_no_subscription_with_the_plan() {
    let error = map_period_usage(&raw(FREE), Some("free")).unwrap_err();
    assert_eq!(
        error,
        ProviderError::NoSubscription {
            detail: "No active Cursor subscription (Free plan).".into()
        }
    );
    let missing = map_period_usage(&raw(r#"{"enabled":true}"#), None).unwrap_err();
    assert_eq!(
        missing,
        ProviderError::NoSubscription {
            detail: "No active Cursor subscription.".into()
        }
    );
}

#[test]
fn plan_usage_without_limit_or_percent_needs_request_counts() {
    assert_eq!(
        map_period_usage(&raw(REQUEST_BASED), Some("enterprise")).unwrap(),
        PlanUsage::RequestBased
    );
}

#[test]
fn percent_falls_back_to_spend_and_remaining() {
    let usage = mapped(r#"{"planUsage":{"limit":"2000","remaining":"1500"}}"#, None);
    assert_eq!(usage.windows[0].used, Percent::new(25.0));
    assert_eq!(usage.windows[0].resets_at, None);
    assert_eq!(usage.windows[0].period, None);
}

#[test]
fn zero_limit_without_percent_shows_no_total() {
    let usage = mapped(r#"{"planUsage":{"limit":0,"autoPercentUsed":5}}"#, None);
    assert_eq!(
        usage.windows,
        [window(
            "auto",
            "Auto",
            Percent::new(5.0),
            Cycle {
                resets_at: None,
                period: None
            }
        )]
    );
}

#[test]
fn inverted_cycle_keeps_reset_without_period() {
    let usage = mapped(
        r#"{"billingCycleStart":2000,"billingCycleEnd":1000,"planUsage":{"totalPercentUsed":1}}"#,
        None,
    );
    assert_eq!(
        usage.windows[0].resets_at,
        Some(Timestamp::from_millisecond(1000).unwrap())
    );
    assert_eq!(usage.windows[0].period, None);
}

#[test]
fn on_demand_spend_rules() {
    let spend = |value: serde_json::Value| -> Vec<Balance> {
        on_demand_balances(&serde_json::from_value(value).unwrap())
    };
    assert_eq!(
        spend(json!({ "individualLimit": 1000, "individualRemaining": 400 })),
        [
            usd_balance("on_demand_spent", "On-demand spent", MicroUsd(6_000_000)),
            usd_balance("on_demand_limit", "On-demand limit", MicroUsd(10_000_000)),
        ]
    );
    assert_eq!(
        spend(json!({ "totalSpend": 250 })),
        [usd_balance(
            "on_demand_spent",
            "On-demand spent",
            MicroUsd(2_500_000)
        )]
    );
    assert!(spend(json!({ "individualLimit": 0, "totalSpend": 0 })).is_empty());
    assert!(spend(json!({})).is_empty());
}

#[test]
fn plan_labels_are_title_cased() {
    assert_eq!(plan_label("pro plan").as_deref(), Some("Pro Plan"));
    assert_eq!(plan_label("free_trial").as_deref(), Some("Free Trial"));
    assert_eq!(plan_label("ULTRA").as_deref(), Some("Ultra"));
    assert_eq!(plan_label("  "), None);
}

#[test]
fn wrong_types_in_optional_numbers_are_ignored() {
    let usage = mapped(
        r#"{"planUsage":{"totalPercentUsed":10,"autoPercentUsed":true,"apiPercentUsed":"n/a"},"extra":[1]}"#,
        None,
    );
    assert_eq!(usage.windows.len(), 1);
}
