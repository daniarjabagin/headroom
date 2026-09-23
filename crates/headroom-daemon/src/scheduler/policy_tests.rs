use super::*;
use crate::testing::ts;

const MID: f64 = 0.5;

fn secs(value: i64) -> SignedDuration {
    SignedDuration::from_secs(value)
}

#[test]
fn jitter_stays_within_ten_percent() {
    assert_eq!(jittered(secs(300), MID), secs(300));
    assert_eq!(jittered(secs(300), 0.0), secs(270));
    assert_eq!(jittered(secs(300), 1.0), secs(330));
    assert_eq!(jittered(secs(300), 7.0), secs(330));
}

#[test]
fn backoff_doubles_from_one_minute_to_the_cap() {
    let delays: Vec<i64> = (1..=8).map(|n| backoff(n, MID).as_secs()).collect();
    assert_eq!(delays, [60, 120, 240, 480, 960, 1_800, 1_800, 1_800]);
    assert_eq!(backoff(0, MID), secs(60));
    assert_eq!(backoff(u32::MAX, MID), BACKOFF_CAP);
}

#[test]
fn backoff_jitter_never_exceeds_the_cap() {
    assert_eq!(backoff(1, 0.0), secs(54));
    assert_eq!(backoff(1, 1.0), secs(66));
    assert_eq!(backoff(6, 1.0), BACKOFF_CAP);
    assert_eq!(backoff(5, 1.0), secs(1_056));
}

#[test]
fn retry_after_is_honoured_with_a_default() {
    assert_eq!(rate_limit_delay(Some(secs(120))), secs(120));
    assert_eq!(rate_limit_delay(None), secs(300));
    assert_eq!(rate_limit_delay(Some(secs(0))), secs(300));
    assert_eq!(rate_limit_delay(Some(secs(-5))), secs(300));
}

#[test]
fn next_delay_depends_on_the_outcome() {
    let interval = secs(300);
    let limited = RefreshFailure::Provider(ProviderError::RateLimited {
        retry_after: Some(secs(90)),
    });
    let network = RefreshFailure::Provider(ProviderError::Network("down".into()));
    assert_eq!(next_delay(Ok(()), 0, interval, MID), interval);
    assert_eq!(next_delay(Err(&limited), 3, interval, MID), secs(90));
    assert_eq!(next_delay(Err(&network), 3, interval, MID), secs(240));
    assert_eq!(
        next_delay(Err(&RefreshFailure::Timeout), 1, interval, MID),
        secs(60)
    );
    assert!(is_rate_limited(&limited));
    assert!(!is_rate_limited(&network));
}

#[test]
fn initial_delay_waits_out_a_fresh_cache() {
    let now = ts("2026-09-23T10:00:00Z");
    let interval = secs(300);
    assert_eq!(initial_delay(None, now, interval), SignedDuration::ZERO);
    assert_eq!(
        initial_delay(Some(ts("2026-09-23T09:58:00Z")), now, interval),
        secs(180)
    );
    assert_eq!(
        initial_delay(Some(ts("2026-09-23T09:00:00Z")), now, interval),
        SignedDuration::ZERO
    );
}

#[test]
fn soft_refresh_skips_recent_refreshing_and_held_accounts() {
    let now = ts("2026-09-23T10:00:00Z");
    let at = |text: &str| AccountRuntime {
        last_attempt: Some(ts(text)),
        ..AccountRuntime::default()
    };
    assert!(soft_refresh_due(None, now));
    assert!(soft_refresh_due(Some(&at("2026-09-23T09:59:00Z")), now));
    assert!(!soft_refresh_due(Some(&at("2026-09-23T09:59:01Z")), now));
    let refreshing = AccountRuntime {
        refreshing: true,
        ..AccountRuntime::default()
    };
    assert!(!soft_refresh_due(Some(&refreshing), now));
    let held = AccountRuntime {
        hold_until: Some(ts("2026-09-23T10:01:00Z")),
        ..at("2026-09-23T09:00:00Z")
    };
    assert!(!soft_refresh_due(Some(&held), now));
}
