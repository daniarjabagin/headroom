use headroom_core::provider::ProviderError;
use jiff::{SignedDuration, Timestamp};

use crate::model::{AccountRuntime, RefreshFailure};
use crate::settings::Settings;

pub const FETCH_TIMEOUT: SignedDuration = SignedDuration::from_secs(30);
pub const SOFT_REFRESH_AFTER: SignedDuration = SignedDuration::from_secs(60);
pub const BACKOFF_BASE: SignedDuration = SignedDuration::from_secs(60);
pub const BACKOFF_CAP: SignedDuration = SignedDuration::from_mins(30);
pub const RATE_LIMIT_BASE: SignedDuration = SignedDuration::from_mins(5);
pub const RATE_LIMIT_CAP: SignedDuration = SignedDuration::from_hours(1);
pub const MANUAL_RETRY_SPACING: SignedDuration = SignedDuration::from_secs(60);
pub const MIN_INTERVAL: SignedDuration = SignedDuration::from_secs(60);
pub const LIVE_INTERVAL: SignedDuration = MIN_INTERVAL;
pub const NO_SUBSCRIPTION_RECHECK: SignedDuration = SignedDuration::from_hours(1);
const JITTER: f64 = 0.1;
const MAX_DOUBLINGS: u32 = 16;

#[must_use]
pub fn jittered(base: SignedDuration, sample: f64) -> SignedDuration {
    let factor = 1.0 + JITTER * (2.0 * sample.clamp(0.0, 1.0) - 1.0);
    base.mul_f64(factor)
}

#[must_use]
pub fn backoff(failures: u32, sample: f64) -> SignedDuration {
    doubling(BACKOFF_BASE, BACKOFF_CAP, failures, sample)
}

#[must_use]
pub fn rate_limit_delay(
    retry_after: Option<SignedDuration>,
    streak: u32,
    sample: f64,
) -> SignedDuration {
    retry_after.filter(SignedDuration::is_positive).map_or_else(
        || doubling(RATE_LIMIT_BASE, RATE_LIMIT_CAP, streak, sample),
        |wait| wait.min(RATE_LIMIT_CAP),
    )
}

fn doubling(base: SignedDuration, cap: SignedDuration, streak: u32, sample: f64) -> SignedDuration {
    let doublings = streak.saturating_sub(1).min(MAX_DOUBLINGS);
    let multiplier = 1_i32 << doublings;
    let delay = base.checked_mul(multiplier).unwrap_or(cap).min(cap);
    jittered(delay, sample).min(cap)
}

#[must_use]
pub fn failure_streak(runtime: Option<&AccountRuntime>, failure: &RefreshFailure) -> u32 {
    let previous = runtime.map_or(0, |runtime| {
        if failure.is_rate_limited() {
            runtime.rate_limits
        } else {
            runtime.failures
        }
    });
    previous.saturating_add(1)
}

#[must_use]
pub fn next_delay(
    outcome: Result<(), &RefreshFailure>,
    streak: u32,
    interval: SignedDuration,
    sample: f64,
) -> SignedDuration {
    match outcome {
        Ok(()) => jittered(interval, sample).max(MIN_INTERVAL),
        Err(RefreshFailure::Provider(ProviderError::RateLimited { retry_after, .. })) => {
            rate_limit_delay(*retry_after, streak, sample)
        }
        Err(RefreshFailure::Provider(ProviderError::NoSubscription { .. })) => {
            jittered(NO_SUBSCRIPTION_RECHECK, sample)
        }
        Err(_) => backoff(streak, sample),
    }
}

#[must_use]
pub fn adaptive_live(settings: &Settings, live: bool) -> bool {
    live && settings.adaptive_refresh
}

#[must_use]
pub fn effective_interval(settings: &Settings, live: bool) -> SignedDuration {
    let interval = if adaptive_live(settings, live) {
        LIVE_INTERVAL
    } else {
        settings.refresh_interval()
    };
    interval.max(MIN_INTERVAL)
}

#[must_use]
pub fn provider_interval(
    settings: &Settings,
    live: bool,
    min_poll: Option<SignedDuration>,
) -> SignedDuration {
    effective_interval(settings, live).max(provider_floor(min_poll))
}

#[must_use]
pub fn provider_floor(min_poll: Option<SignedDuration>) -> SignedDuration {
    min_poll.unwrap_or(SignedDuration::ZERO)
}

#[must_use]
pub fn live_delay(
    runtime: Option<&AccountRuntime>,
    interval: SignedDuration,
    now: Timestamp,
) -> Option<SignedDuration> {
    let Some(runtime) = runtime else {
        return Some(SignedDuration::ZERO);
    };
    let held = runtime.hold_until.is_some_and(|until| until > now);
    if runtime.refreshing || runtime.failure.is_some() || held {
        return None;
    }
    Some(initial_delay(runtime.last_attempt, now, interval))
}

#[must_use]
pub fn holds_soft_refresh(failure: &RefreshFailure) -> bool {
    matches!(
        failure,
        RefreshFailure::Provider(
            ProviderError::RateLimited { .. } | ProviderError::NoSubscription { .. }
        )
    )
}

#[must_use]
pub fn initial_delay(
    last_fetched: Option<Timestamp>,
    now: Timestamp,
    interval: SignedDuration,
) -> SignedDuration {
    last_fetched.map_or(SignedDuration::ZERO, |fetched| {
        (interval - now.duration_since(fetched)).max(SignedDuration::ZERO)
    })
}

#[must_use]
pub fn soft_refresh_due(runtime: Option<&AccountRuntime>, now: Timestamp) -> bool {
    let Some(runtime) = runtime else {
        return true;
    };
    let held = runtime.hold_until.is_some_and(|until| until > now);
    let recent = runtime
        .last_attempt
        .is_some_and(|at| now.duration_since(at) < SOFT_REFRESH_AFTER);
    !runtime.refreshing && !held && !recent
}

#[must_use]
pub fn forced_refresh_allowed(runtime: Option<&AccountRuntime>, now: Timestamp) -> bool {
    runtime.is_none_or(|runtime| {
        let held = runtime.hold_until.is_some_and(|until| until > now);
        let rate_limited = runtime
            .failure
            .as_ref()
            .is_some_and(RefreshFailure::is_rate_limited);
        let recent = runtime
            .last_attempt
            .is_some_and(|at| now.duration_since(at) < MANUAL_RETRY_SPACING);
        !(held && rate_limited && (runtime.refreshing || recent))
    })
}

#[cfg(test)]
#[path = "policy_tests.rs"]
mod tests;
