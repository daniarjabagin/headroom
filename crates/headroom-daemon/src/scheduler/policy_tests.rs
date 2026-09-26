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
fn retry_after_is_honoured_over_the_streak() {
    assert_eq!(rate_limit_delay(Some(secs(120)), 1, MID), secs(120));
    assert_eq!(rate_limit_delay(Some(secs(120)), 4, 0.0), secs(120));
    assert_eq!(rate_limit_delay(None, 1, MID), secs(300));
    assert_eq!(rate_limit_delay(Some(secs(0)), 1, MID), secs(300));
    assert_eq!(rate_limit_delay(Some(secs(-5)), 1, MID), secs(300));
}

#[test]
fn repeated_rate_limits_without_retry_after_double_up_to_an_hour() {
    let delays: Vec<i64> = (1..=7)
        .map(|streak| rate_limit_delay(None, streak, MID).as_mins())
        .collect();
    assert_eq!(delays, [5, 10, 20, 40, 60, 60, 60]);
    assert_eq!(rate_limit_delay(None, 0, MID), secs(300));
    assert_eq!(rate_limit_delay(None, u32::MAX, MID), RATE_LIMIT_CAP);
}

#[test]
fn rate_limit_backoff_is_jittered_below_the_cap() {
    assert_eq!(rate_limit_delay(None, 1, 0.0), secs(270));
    assert_eq!(rate_limit_delay(None, 2, 1.0), secs(660));
    assert_eq!(rate_limit_delay(None, 5, 1.0), RATE_LIMIT_CAP);
    assert_eq!(rate_limit_delay(None, 5, 0.0), secs(3_240));
}

#[test]
fn the_rate_limit_streak_counts_only_consecutive_rate_limits() {
    let limited = RefreshFailure::Provider(ProviderError::rate_limited(None));
    let network = RefreshFailure::Provider(ProviderError::Network("down".into()));
    let runtime = AccountRuntime {
        failures: 5,
        rate_limits: 2,
        ..AccountRuntime::default()
    };
    assert_eq!(failure_streak(None, &limited), 1);
    assert_eq!(failure_streak(Some(&runtime), &limited), 3);
    assert_eq!(failure_streak(Some(&runtime), &network), 6);
}

#[test]
fn retry_after_is_capped_at_one_hour() {
    let far_future: Timestamp = "9999-01-01T00:00:00Z".parse().unwrap();
    let far_wait = far_future.duration_since(ts("2026-09-23T10:00:00Z"));
    let cases = [
        (secs(3_599), secs(3_599)),
        (secs(3_600), secs(3_600)),
        (secs(3_601), RATE_LIMIT_CAP),
        (secs(4_294_967_295), RATE_LIMIT_CAP),
        (far_wait, RATE_LIMIT_CAP),
    ];
    for (retry_after, expected) in cases {
        assert_eq!(
            rate_limit_delay(Some(retry_after), 1, MID),
            expected,
            "{retry_after}"
        );
    }
    assert_eq!(RATE_LIMIT_CAP, SignedDuration::from_hours(1));
}

#[test]
fn next_delay_depends_on_the_outcome() {
    let interval = secs(300);
    let limited = RefreshFailure::Provider(ProviderError::rate_limited(Some(secs(90))));
    let network = RefreshFailure::Provider(ProviderError::Network("down".into()));
    assert_eq!(next_delay(Ok(()), 0, interval, MID), interval);
    assert_eq!(next_delay(Err(&limited), 3, interval, MID), secs(90));
    assert_eq!(next_delay(Err(&network), 3, interval, MID), secs(240));
    assert_eq!(
        next_delay(Err(&RefreshFailure::Timeout), 1, interval, MID),
        secs(60)
    );
    assert!(holds_soft_refresh(&limited));
    assert!(!holds_soft_refresh(&network));
}

#[test]
fn no_subscription_is_rechecked_hourly_and_holds_soft_refreshes() {
    let lapsed = RefreshFailure::Provider(ProviderError::NoSubscription {
        detail: "none".into(),
    });
    let interval = secs(300);
    assert_eq!(next_delay(Err(&lapsed), 1, interval, MID), secs(3_600));
    assert_eq!(next_delay(Err(&lapsed), 9, interval, 0.0), secs(3_240));
    assert!(holds_soft_refresh(&lapsed));
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

#[test]
fn forced_refresh_ignores_recency_and_lapses_but_keeps_rate_limit_holds() {
    let now = ts("2026-09-23T10:00:00Z");
    let failing = |failure: ProviderError, hold_until: &str| AccountRuntime {
        last_attempt: Some(ts("2026-09-23T09:59:50Z")),
        failure: Some(RefreshFailure::Provider(failure)),
        hold_until: Some(ts(hold_until)),
        ..AccountRuntime::default()
    };
    let limited = || ProviderError::rate_limited(None);
    let lapsed = ProviderError::NoSubscription {
        detail: "free plan".into(),
    };
    let recent = AccountRuntime {
        last_attempt: Some(ts("2026-09-23T09:59:50Z")),
        refreshing: true,
        ..AccountRuntime::default()
    };
    assert!(forced_refresh_allowed(None, now));
    assert!(forced_refresh_allowed(Some(&recent), now));
    assert!(forced_refresh_allowed(
        Some(&failing(lapsed, "2026-09-23T11:00:00Z")),
        now
    ));
    assert!(!forced_refresh_allowed(
        Some(&failing(limited(), "2026-09-23T10:02:00Z")),
        now
    ));
    assert!(forced_refresh_allowed(
        Some(&failing(limited(), "2026-09-23T10:00:00Z")),
        now
    ));
}

#[test]
fn a_manual_retry_during_a_rate_limit_is_allowed_once_a_minute() {
    let now = ts("2026-09-23T10:00:00Z");
    let limited = |last_attempt: &str, refreshing: bool| AccountRuntime {
        last_attempt: Some(ts(last_attempt)),
        failure: Some(RefreshFailure::Provider(ProviderError::rate_limited(None))),
        hold_until: Some(ts("2026-09-23T10:20:00Z")),
        refreshing,
        ..AccountRuntime::default()
    };
    assert!(forced_refresh_allowed(
        Some(&limited("2026-09-23T09:59:00Z", false)),
        now
    ));
    assert!(!forced_refresh_allowed(
        Some(&limited("2026-09-23T09:59:01Z", false)),
        now
    ));
    assert!(!forced_refresh_allowed(
        Some(&limited("2026-09-23T09:50:00Z", true)),
        now
    ));
}

#[test]
fn a_provider_minimum_raises_every_interval() {
    let settings = Settings::default();
    let floor = Some(secs(180));
    assert_eq!(provider_interval(&settings, true, None), LIVE_INTERVAL);
    assert_eq!(provider_interval(&settings, true, floor), secs(180));
    assert_eq!(
        provider_interval(&settings, false, floor),
        settings.refresh_interval().max(secs(180))
    );
    assert_eq!(provider_floor(None), SignedDuration::ZERO);
}
