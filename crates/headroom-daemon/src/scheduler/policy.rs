use headroom_core::provider::ProviderError;
use jiff::{SignedDuration, Timestamp};

use crate::model::{AccountRuntime, RefreshFailure};

pub const FETCH_TIMEOUT: SignedDuration = SignedDuration::from_secs(30);
pub const SOFT_REFRESH_AFTER: SignedDuration = SignedDuration::from_secs(60);
pub const BACKOFF_BASE: SignedDuration = SignedDuration::from_secs(60);
pub const BACKOFF_CAP: SignedDuration = SignedDuration::from_mins(30);
pub const RATE_LIMIT_DEFAULT: SignedDuration = SignedDuration::from_mins(5);
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
    let doublings = failures.saturating_sub(1).min(MAX_DOUBLINGS);
    let multiplier = 1_i32 << doublings;
    let base = BACKOFF_BASE
        .checked_mul(multiplier)
        .unwrap_or(BACKOFF_CAP)
        .min(BACKOFF_CAP);
    jittered(base, sample).min(BACKOFF_CAP)
}

#[must_use]
pub fn rate_limit_delay(retry_after: Option<SignedDuration>) -> SignedDuration {
    retry_after
        .filter(SignedDuration::is_positive)
        .unwrap_or(RATE_LIMIT_DEFAULT)
}

#[must_use]
pub fn next_delay(
    outcome: Result<(), &RefreshFailure>,
    failures: u32,
    interval: SignedDuration,
    sample: f64,
) -> SignedDuration {
    match outcome {
        Ok(()) => jittered(interval, sample),
        Err(RefreshFailure::Provider(ProviderError::RateLimited { retry_after })) => {
            rate_limit_delay(*retry_after)
        }
        Err(RefreshFailure::Provider(ProviderError::NoSubscription { .. })) => {
            jittered(NO_SUBSCRIPTION_RECHECK, sample)
        }
        Err(_) => backoff(failures, sample),
    }
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

#[cfg(test)]
#[path = "policy_tests.rs"]
mod tests;
