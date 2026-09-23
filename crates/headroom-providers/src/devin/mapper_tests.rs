use serde_json::json;

use super::super::raw::RawStatusResponse;
use super::*;

const FULL: &str = include_str!("fixtures/user_status.json");
const NO_PLAN: &str = include_str!("fixtures/user_status_no_plan.json");
const WEEKLY_SPENT: &str = include_str!("fixtures/user_status_weekly_spent.json");

fn map(body: &str) -> Result<MappedStatus, ProviderError> {
    let raw: RawStatusResponse = serde_json::from_str(body).unwrap();
    map_status(&raw.user_status)
}

fn map_plan_status(plan_status: &serde_json::Value) -> Result<MappedStatus, ProviderError> {
    map(&json!({ "userStatus": { "planStatus": plan_status } }).to_string())
}

fn at(seconds: i64) -> Timestamp {
    Timestamp::from_second(seconds).unwrap()
}

#[test]
fn the_full_status_maps_daily_weekly_and_the_exact_overage_balance() {
    let mapped = map(FULL).unwrap();
    assert_eq!(mapped.plan.as_deref(), Some("Max"));
    assert_eq!(mapped.email.as_deref(), Some("user@example.test"));
    assert_eq!(
        mapped.windows,
        [
            QuotaWindow {
                id: WindowId::Other("daily".into()),
                label: "Daily".into(),
                used: Percent::new(0.0),
                resets_at: Some(at(1_774_080_000)),
                period: Some(SignedDuration::from_hours(24)),
            },
            QuotaWindow {
                id: WindowId::Weekly,
                label: "Weekly".into(),
                used: Percent::new(60.0),
                resets_at: Some(at(1_774_166_400)),
                period: Some(WindowId::WEEKLY_PERIOD),
            },
        ]
    );
    assert_eq!(
        mapped.balances,
        [Balance {
            id: "extra_usage".into(),
            label: "Extra usage balance".into(),
            amount: BalanceAmount::Usd(MicroUsd(964_220_000)),
        }]
    );
}

#[test]
fn a_weekly_reset_without_a_percentage_means_the_week_is_spent() {
    let mapped = map(WEEKLY_SPENT).unwrap();
    assert_eq!(mapped.windows.len(), 1);
    let weekly = &mapped.windows[0];
    assert_eq!(weekly.id, WindowId::Weekly);
    assert_eq!(weekly.used, Percent::FULL);
    assert_eq!(weekly.resets_at, Some(at(1_789_286_400)));
    assert_eq!(
        mapped.balances[0].amount,
        BalanceAmount::Usd(MicroUsd::ZERO)
    );
}

#[test]
fn a_hidden_daily_quota_is_left_out() {
    let mapped = map_plan_status(&json!({
        "planInfo": { "planName": "Pro", "hideDailyQuota": "true" },
        "dailyQuotaRemainingPercent": 30,
        "weeklyQuotaRemainingPercent": "12.5"
    }))
    .unwrap();
    let ids: Vec<_> = mapped.windows.iter().map(|w| w.id.clone()).collect();
    assert_eq!(ids, [WindowId::Weekly]);
    assert_eq!(mapped.windows[0].used, Percent::new(87.5));
    assert_eq!(mapped.windows[0].resets_at, None);
}

#[test]
fn numbers_may_arrive_as_strings_or_numbers() {
    let mapped = map_plan_status(&json!({
        "dailyQuotaRemainingPercent": "25",
        "dailyQuotaResetAtUnix": 1_774_080_000,
        "overageBalanceMicros": -44_347
    }))
    .unwrap();
    assert_eq!(mapped.plan, None);
    assert_eq!(mapped.windows[0].used, Percent::new(75.0));
    assert_eq!(mapped.windows[0].resets_at, Some(at(1_774_080_000)));
    assert_eq!(
        mapped.balances[0].amount,
        BalanceAmount::Usd(MicroUsd(-44_347))
    );
}

#[test]
fn no_quota_and_no_balance_is_no_subscription() {
    let expected = ProviderError::NoSubscription {
        detail: "No active Devin plan.".into(),
    };
    assert_eq!(map(NO_PLAN), Err(expected.clone()));
    assert_eq!(map(r#"{"userStatus":{}}"#), Err(expected));
}

#[test]
fn present_but_unreadable_fields_fail_loudly() {
    for plan_status in [
        json!({ "weeklyQuotaRemainingPercent": "abc", "weeklyQuotaResetAtUnix": "1789286400" }),
        json!({ "weeklyQuotaRemainingPercent": true }),
        json!({ "dailyQuotaRemainingPercent": 10, "dailyQuotaResetAtUnix": "soon" }),
        json!({ "overageBalanceMicros": "12.5" }),
        json!({ "planInfo": { "hideDailyQuota": "maybe" }, "overageBalanceMicros": "1" }),
    ] {
        assert!(
            matches!(
                map_plan_status(&plan_status),
                Err(ProviderError::InvalidResponse(_))
            ),
            "{plan_status}"
        );
    }
}
