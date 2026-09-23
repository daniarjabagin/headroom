use headroom_core::units::MicroUsd;
use serde_json::json;

use super::*;

const PRO: &str = include_str!("fixtures/user_pro.json");
const FREE: &str = include_str!("fixtures/user_free.json");
const BUSINESS_SEAT: &str = include_str!("fixtures/user_business_seat.json");
const NO_PLAN_USER: &str = include_str!("fixtures/user_no_plan.json");
const LEGACY_FREE: &str = include_str!("fixtures/user_legacy_free.json");

fn map(body: &str) -> Result<MappedUser, ProviderError> {
    map_user(&serde_json::from_str(body).unwrap())
}

fn ts(text: &str) -> Timestamp {
    text.parse().unwrap()
}

fn used(mapped: &MappedUser) -> Vec<(String, Percent)> {
    mapped
        .windows
        .iter()
        .map(|window| (window.label.clone(), window.used))
        .collect()
}

#[test]
fn a_paid_plan_meters_credits_and_its_extra_usage_in_exact_dollars() {
    let mapped = map(PRO).unwrap();
    assert_eq!(mapped.plan.as_deref(), Some("Individual Pro"));
    assert_eq!(
        mapped.windows,
        [QuotaWindow {
            id: WindowId::Other("credits".into()),
            label: "Credits".into(),
            used: Percent::new(62.5),
            resets_at: Some(ts("2026-10-01T00:00:00Z")),
            period: Some(SignedDuration::from_hours(30 * 24)),
        }]
    );
    assert_eq!(
        mapped.balances,
        [Balance {
            id: "extra_usage".into(),
            label: "Extra usage".into(),
            amount: BalanceAmount::Usd(MicroUsd(125_000)),
        }]
    );
    assert!(mapped.notices.is_empty());
}

#[test]
fn a_free_plan_meters_chat_and_completions_only() {
    let mapped = map(FREE).unwrap();
    assert_eq!(mapped.plan.as_deref(), Some("Individual"));
    let chat = Percent::new(100.0 - 91.0);
    let completions = Percent::new(100.0 - 99.45);
    assert_eq!(
        used(&mapped),
        [("Chat".into(), chat), ("Completions".into(), completions)]
    );
    assert!(mapped.balances.is_empty());
}

#[test]
fn the_legacy_free_shape_counts_remaining_against_the_monthly_limit() {
    let mapped = map(LEGACY_FREE).unwrap();
    assert_eq!(
        used(&mapped),
        [
            ("Chat".into(), Percent::new(50.0)),
            ("Completions".into(), Percent::new(50.0))
        ]
    );
    let window = &mapped.windows[0];
    assert_eq!(window.resets_at, Some(ts("2026-10-15T00:00:00Z")));
    assert_eq!(window.period, Some(SignedDuration::from_hours(30 * 24)));
}

#[test]
fn an_org_seat_shows_the_personal_credits_used_and_explains_the_rest() {
    let mapped = map(BUSINESS_SEAT).unwrap();
    assert_eq!(mapped.plan.as_deref(), Some("Business"));
    assert!(mapped.windows.is_empty());
    assert_eq!(
        mapped.balances,
        [Balance {
            id: "credits_used".into(),
            label: "Credits used".into(),
            amount: BalanceAmount::Usd(MicroUsd(21_110_000)),
        }]
    );
    assert_eq!(mapped.notices.len(), 1);
    assert_eq!(mapped.notices[0].tone, Tone::Neutral);
}

#[test]
fn an_unused_org_seat_keeps_the_plan_without_a_zero_balance() {
    let mut body: serde_json::Value = serde_json::from_str(BUSINESS_SEAT).unwrap();
    body["quota_snapshots"]["premium_interactions"]["credits_used"] = json!(0);
    let mapped = map(&body.to_string()).unwrap();
    assert!(mapped.balances.is_empty());
    assert_eq!(mapped.notices.len(), 1);
}

#[test]
fn no_quota_without_token_billing_is_no_subscription() {
    assert_eq!(
        map(NO_PLAN_USER),
        Err(ProviderError::NoSubscription {
            detail: "No GitHub Copilot plan on this account.".into()
        })
    );
}

#[test]
fn extra_usage_needs_both_a_credit_pool_and_permission() {
    let mut body: serde_json::Value = serde_json::from_str(PRO).unwrap();
    body["quota_snapshots"]["premium_interactions"]["overage_permitted"] = json!(false);
    assert!(map(&body.to_string()).unwrap().balances.is_empty());
    body["quota_snapshots"]["premium_interactions"]["overage_permitted"] = json!(true);
    body["quota_snapshots"]["premium_interactions"]
        .as_object_mut()
        .unwrap()
        .remove("overage_count");
    assert_eq!(
        map(&body.to_string()).unwrap().balances[0].amount,
        BalanceAmount::Usd(MicroUsd::ZERO)
    );
}

#[test]
fn without_a_percentage_the_counts_give_the_share_used() {
    let body = json!({
        "copilot_plan": "pro",
        "quota_snapshots": { "premium_requests": { "entitlement": 300, "remaining": 75 } }
    });
    let mapped = map(&body.to_string()).unwrap();
    assert_eq!(used(&mapped), [("Credits".into(), Percent::new(75.0))]);
    assert_eq!(mapped.windows[0].resets_at, None);
    assert_eq!(mapped.windows[0].period, None);
}

#[test]
fn reset_timestamps_and_dates_set_the_calendar_month() {
    let body = json!({
        "quota_reset_date": "2026-03-01T00:00:00Z",
        "quota_snapshots": { "chat": { "entitlement": 10, "remaining": 5 } }
    });
    let window = &map(&body.to_string()).unwrap().windows[0];
    assert_eq!(window.period, Some(SignedDuration::from_hours(28 * 24)));
    let broken = json!({ "quota_reset_date": "next month" });
    assert!(matches!(
        map(&broken.to_string()),
        Err(ProviderError::InvalidResponse(_))
    ));
}

#[test]
fn unreadable_credit_counts_fail_loudly() {
    let mut body: serde_json::Value = serde_json::from_str(PRO).unwrap();
    body["quota_snapshots"]["premium_interactions"]["overage_count"] = json!(1e30);
    assert!(matches!(
        map(&body.to_string()),
        Err(ProviderError::InvalidResponse(_))
    ));
}

#[test]
fn plan_names_are_title_cased() {
    assert_eq!(
        plan_label("individual_pro").as_deref(),
        Some("Individual Pro")
    );
    assert_eq!(plan_label("ENTERPRISE").as_deref(), Some("Enterprise"));
    assert_eq!(plan_label(" _ ").as_deref(), None);
}
