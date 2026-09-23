use serde_json::json;

use super::*;
use crate::ollama::raw::{RawActivity, RawLimits};

const USAGE: &str = include_str!("fixtures/usage.json");
const USAGE_FREE: &str = include_str!("fixtures/usage_free.json");
const ME: &str = include_str!("fixtures/me.json");

fn now() -> Timestamp {
    "2026-09-23T10:00:00Z".parse().unwrap()
}

fn usage(text: &str) -> RawUsage {
    serde_json::from_str(text).unwrap()
}

fn me() -> RawMe {
    serde_json::from_str(ME).unwrap()
}

fn summary(snapshot: &LimitsSnapshot) -> Vec<(WindowId, String, f64, Option<i64>)> {
    snapshot
        .windows
        .iter()
        .map(|window| {
            (
                window.id.clone(),
                window.label.clone(),
                (window.used.value() * 1000.0).round() / 1000.0,
                window.period.map(|period| period.as_secs()),
            )
        })
        .collect()
}

#[test]
fn fractions_become_percent_windows_without_resets() {
    let snapshot = map_snapshot(&usage(USAGE), Some(&me()), "key".into(), now());
    assert_eq!(
        summary(&snapshot),
        [
            (WindowId::Session, "Session".into(), 34.9, Some(18_000)),
            (WindowId::Weekly, "Weekly".into(), 25.0, Some(604_800)),
            (
                WindowId::Other("monthly".into()),
                "Monthly".into(),
                5.3,
                None
            ),
        ]
    );
    assert!(snapshot.windows.iter().all(|w| w.resets_at.is_none()));
    assert_eq!(snapshot.fetched_at, now());
    assert_eq!(snapshot.source, LimitsSource::Live);
    assert!(snapshot.notices.is_empty());
}

#[test]
fn four_week_cost_is_an_exact_usd_balance() {
    let snapshot = map_snapshot(&usage(USAGE), Some(&me()), "key".into(), now());
    assert_eq!(
        snapshot.balances,
        [Balance {
            id: "extra_usage_4w".into(),
            label: "Extra usage (4 weeks)".into(),
            amount: BalanceAmount::Usd(MicroUsd(1_250_000)),
        }]
    );
    let free = map_snapshot(&usage(USAGE_FREE), Some(&me()), "key".into(), now());
    assert_eq!(free.balances[0].amount, BalanceAmount::Usd(MicroUsd(0)));
}

#[test]
fn numeric_costs_are_read_and_odd_costs_are_left_out() {
    let numeric = usage(r#"{"limits":{},"activity":{"cost":2.5}}"#);
    let mapped = map_snapshot(&numeric, None, "key".into(), now());
    assert_eq!(
        mapped.balances[0].amount,
        BalanceAmount::Usd(MicroUsd(2_500_000))
    );
    for cost in [json!("1e3"), json!(null), json!({"usd": 1})] {
        let odd = RawUsage {
            limits: RawLimits::default(),
            activity: Some(RawActivity { cost: Some(cost) }),
        };
        assert!(
            map_snapshot(&odd, None, "key".into(), now())
                .balances
                .is_empty()
        );
    }
}

#[test]
fn identity_carries_email_and_title_cased_plan() {
    let snapshot = map_snapshot(&usage(USAGE), Some(&me()), "key".into(), now());
    assert_eq!(
        snapshot.identity,
        AccountIdentity {
            email: Some("someone@example.com".into()),
            plan: Some("Pro".into()),
            stable_key: "key".into(),
        }
    );
    let lower: RawMe = serde_json::from_str(r#"{"email":"a@example.com","plan":"MAX"}"#).unwrap();
    let mapped = map_snapshot(&usage(USAGE), Some(&lower), "key".into(), now());
    assert_eq!(mapped.identity.plan.as_deref(), Some("Max"));
}

#[test]
fn a_free_account_shows_only_the_limits_it_reports() {
    let snapshot = map_snapshot(&usage(USAGE_FREE), Some(&me()), "key".into(), now());
    assert_eq!(
        summary(&snapshot),
        [(
            WindowId::Other("monthly".into()),
            "Monthly".into(),
            50.0,
            None
        )]
    );
}

#[test]
fn missing_limits_and_a_missing_plan_are_explained() {
    let empty = usage(r#"{"limits":{"session":{},"weekly":{"usage":null}}}"#);
    let snapshot = map_snapshot(&empty, None, "key".into(), now());
    assert!(snapshot.windows.is_empty());
    assert!(snapshot.balances.is_empty());
    let texts: Vec<_> = snapshot.notices.iter().map(|n| n.text.as_str()).collect();
    assert_eq!(texts, [NO_LIMITS_TEXT, NO_PLAN_TEXT]);
    assert_eq!(snapshot.identity.plan, None);
}

#[test]
fn a_response_without_limits_is_not_usage() {
    assert!(serde_json::from_str::<RawUsage>(r#"{"activity":{"cost":"0"}}"#).is_err());
}
