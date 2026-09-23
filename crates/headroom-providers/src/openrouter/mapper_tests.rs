use headroom_core::provider::ProviderError;

use super::super::raw::Envelope;
use super::*;

const KEY: &str = include_str!("fixtures/key.json");
const KEY_LIMITED: &str = include_str!("fixtures/key_limited.json");
const CREDITS: &str = include_str!("fixtures/credits.json");

fn key(body: &str) -> RawKey {
    serde_json::from_str::<Envelope<RawKey>>(body).unwrap().data
}

fn credits() -> Credits {
    Credits::Available(
        serde_json::from_str::<Envelope<RawCredits>>(CREDITS)
            .unwrap()
            .data,
    )
}

fn usd(id: &str, label: &str, micros: i64) -> Balance {
    Balance {
        id: id.into(),
        label: label.into(),
        amount: BalanceAmount::Usd(MicroUsd(micros)),
    }
}

#[test]
fn a_management_key_shows_the_exact_balance_and_spend() {
    let mapped = map(&key(KEY), &credits());
    assert_eq!(mapped.plan.as_deref(), Some("Pay as you go"));
    assert_eq!(mapped.windows, []);
    assert_eq!(mapped.notices, []);
    assert_eq!(
        mapped.balances,
        [
            usd("credits", "Credit balance", 108_123_579),
            usd("spend_today", "Spent today", 0),
            usd("spend_week", "Spent this week", 3_141_593),
            usd("spend_month", "Spent this month", 12_050_000),
        ]
    );
}

#[test]
fn a_regular_key_explains_the_missing_balance() {
    let mapped = map(&key(KEY), &Credits::NeedsManagementKey);
    assert_eq!(
        mapped.notices,
        [Notice {
            tone: Tone::Neutral,
            text: "Credit balance needs a management key".into(),
        }]
    );
    assert_eq!(mapped.balances.len(), 3);
    assert!(mapped.balances.iter().all(|b| b.id.starts_with("spend_")));
}

#[test]
fn a_failed_credit_lookup_degrades_to_a_notice() {
    let failed = Credits::Unavailable(ProviderError::Network("HTTP 502".into()));
    let mapped = map(&key(KEY), &failed);
    assert_eq!(mapped.notices[0].text, CREDITS_UNAVAILABLE_NOTICE);
    assert_eq!(mapped.notices[0].tone, Tone::Neutral);
}

#[test]
fn a_limited_key_is_a_percent_window_without_reset() {
    let mapped = map(&key(KEY_LIMITED), &Credits::NeedsManagementKey);
    assert_eq!(mapped.plan.as_deref(), Some("Free tier"));
    assert_eq!(
        mapped.windows,
        [QuotaWindow {
            id: WindowId::Other("key_limit".into()),
            label: "Key limit".into(),
            used: Percent::new(62.5),
            resets_at: None,
            period: None,
        }]
    );
}

#[test]
fn key_limit_edges() {
    let base = key(KEY_LIMITED);
    let over = RawKey {
        limit_remaining: Some(Usd(MicroUsd(-5_000_000))),
        ..base.clone()
    };
    assert_eq!(key_limit_window(&over).unwrap().used, Percent::new(125.0));
    let spare = RawKey {
        limit_remaining: Some(Usd(MicroUsd(30_000_000))),
        ..base.clone()
    };
    assert_eq!(key_limit_window(&spare).unwrap().used, Percent::ZERO);
    let lifetime = RawKey {
        limit_remaining: None,
        limit_reset: None,
        usage: Some(Usd(MicroUsd(5_000_000))),
        ..base.clone()
    };
    assert_eq!(
        key_limit_window(&lifetime).unwrap().used,
        Percent::new(25.0)
    );
    let unknown_window = RawKey {
        limit_remaining: None,
        ..base.clone()
    };
    assert_eq!(key_limit_window(&unknown_window), None);
    let zero = RawKey {
        limit: Some(Usd(MicroUsd(0))),
        ..base
    };
    assert_eq!(key_limit_window(&zero), None);
}

#[test]
fn an_overdrawn_account_shows_a_negative_balance() {
    let overdrawn = Credits::Available(RawCredits {
        total_credits: Usd(MicroUsd(10_000_000)),
        total_usage: Usd(MicroUsd(10_000_001)),
    });
    let mapped = map(&key(KEY), &overdrawn);
    assert_eq!(mapped.balances[0], usd("credits", "Credit balance", -1));
}

#[test]
fn missing_spend_fields_are_left_out_not_zeroed() {
    let bare: RawKey = serde_json::from_str(r#"{"usage": 1}"#).unwrap();
    let mapped = map(&bare, &Credits::NeedsManagementKey);
    assert_eq!(mapped.balances, []);
    assert_eq!(mapped.plan, None);
}
