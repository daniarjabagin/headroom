use jiff::SignedDuration;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};

use super::*;
use crate::cursor::client::parse;

const GRANTS: &str = include_str!("fixtures/credit_grants.json");
const STRIPE: &str = include_str!("fixtures/stripe.json");
const GROK_BOT: &str = include_str!("fixtures/grok_bot.json");
const REQUESTS: &str = include_str!("fixtures/request_usage.json");

fn fixture<T: DeserializeOwned>(text: &str) -> T {
    parse(text.as_bytes()).unwrap()
}

fn from<T: DeserializeOwned>(value: Value) -> T {
    serde_json::from_value(value).unwrap()
}

fn credits_left(balance: Option<Balance>) -> Option<MicroUsd> {
    balance.map(|balance| match balance.amount {
        headroom_core::quota::BalanceAmount::Usd(amount) => amount,
        headroom_core::quota::BalanceAmount::Count { .. }
        | headroom_core::quota::BalanceAmount::Money(_) => panic!("not usd"),
    })
}

#[test]
fn grants_and_prepaid_balance_add_up() {
    let grants: RawCreditGrants = fixture(GRANTS);
    let stripe: RawStripe = fixture(STRIPE);
    let balance = credits(Some(&grants), Some(&stripe)).unwrap();
    assert_eq!(balance.id, "credits");
    assert_eq!(balance.label, "Credits");
    assert_eq!(credits_left(Some(balance)), Some(MicroUsd(20_000_000)));
}

#[test]
fn credits_table() {
    let cases = [
        (
            json!({ "hasCreditGrants": false }),
            json!({ "customerBalance": "-50000" }),
            Some(500_000_000),
        ),
        (
            json!({ "hasCreditGrants": true, "totalCents": 1000, "usedCents": 1500 }),
            json!({}),
            Some(0),
        ),
        (
            json!({ "hasCreditGrants": true, "totalCents": 0, "usedCents": 0 }),
            json!({ "customerBalance": 0 }),
            None,
        ),
        (
            json!({ "hasCreditGrants": true, "totalCents": 1000 }),
            json!({ "customerBalance": 300 }),
            None,
        ),
        (json!({}), json!({ "customerBalance": true }), None),
    ];
    for (grants, stripe, expected) in cases {
        let grants: RawCreditGrants = from(grants.clone());
        let stripe: RawStripe = from(stripe.clone());
        assert_eq!(
            credits_left(credits(Some(&grants), Some(&stripe))),
            expected.map(MicroUsd),
            "{grants:?} {stripe:?}"
        );
    }
    assert_eq!(credits(None, None), None);
}

#[test]
fn grok_bot_is_a_weekly_window_of_its_own() {
    let window = grok_bot_window(&fixture(GROK_BOT)).unwrap();
    assert_eq!(
        window.id,
        headroom_core::quota::WindowId::Other("grok_bot".into())
    );
    assert_eq!(window.label, "Grok Bot");
    assert_eq!(window.used, Percent::new(37.5));
    assert_eq!(
        window.resets_at,
        Some("2026-09-27T00:00:00Z".parse().unwrap())
    );
    assert_eq!(window.period, Some(SignedDuration::from_hours(7 * 24)));
}

#[test]
fn grok_bot_without_a_personal_allowance_is_skipped() {
    for value in [
        json!({ "usagePercent": 10, "usesPooledEnterpriseAllowance": true }),
        json!({ "usagePercent": 10, "hasNonZeroIncludedLimit": false }),
        json!({ "usagePercent": 10, "includedLimitZero": true }),
        json!({ "usagePercent": -1 }),
        json!({ "usagePercent": true }),
        json!({}),
    ] {
        assert_eq!(grok_bot_window(&from(value.clone())), None, "{value}");
    }
}

#[test]
fn grok_bot_overage_and_missing_dates_are_kept_honest() {
    let window = grok_bot_window(&from(json!({ "usagePercent": 125 }))).unwrap();
    assert_eq!(window.used, Percent::new(125.0));
    assert_eq!(window.resets_at, None);
    assert_eq!(window.period, None);
}

#[test]
fn request_counts_become_a_monthly_window() {
    let window = request_window(&fixture(REQUESTS)).unwrap();
    assert_eq!(window.label, "Requests");
    assert!((window.used.value() - 24.6).abs() < 1e-9);
    assert_eq!(
        window.resets_at,
        Some("2026-10-05T12:00:00Z".parse().unwrap())
    );
    assert_eq!(window.period, Some(SignedDuration::from_hours(30 * 24)));
}

#[test]
fn request_counts_need_a_limit_and_a_count() {
    for value in [
        json!({ "gpt-4": { "maxRequestUsage": 0, "numRequests": 1 } }),
        json!({ "gpt-4": { "maxRequestUsage": 500 } }),
        json!({ "gpt-4": null }),
        json!({}),
    ] {
        assert_eq!(request_window(&from(value.clone())), None, "{value}");
    }
    let total_only = json!({ "gpt-4": { "maxRequestUsage": 200, "numRequestsTotal": 50 } });
    let window = request_window(&from(total_only)).unwrap();
    assert_eq!(window.used, Percent::new(25.0));
    assert_eq!(window.resets_at, None);
}
