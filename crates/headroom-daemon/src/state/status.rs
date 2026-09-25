use headroom_core::provider::ProviderError;
use headroom_core::quota::LimitsSource;
use jiff::{SignedDuration, Timestamp};

use super::payload::{AccountError, AccountStatus, DataSource};
use crate::model::{AccountRuntime, RefreshFailure, SnapshotEntry, SnapshotOrigin};

pub const STALE_AFTER: SignedDuration = SignedDuration::from_mins(10);

#[must_use]
pub fn status(
    runtime: Option<&AccountRuntime>,
    snapshot: Option<&SnapshotEntry>,
    now: Timestamp,
) -> AccountStatus {
    if runtime.is_some_and(|r| r.refreshing) {
        return AccountStatus::Refreshing;
    }
    match runtime.and_then(|r| r.failure.as_ref()) {
        Some(failure) if failure.is_no_subscription() => AccountStatus::NoSubscription,
        Some(failure) if failure.is_signed_out() => AccountStatus::SignedOut,
        Some(_) => AccountStatus::Error,
        None if snapshot.is_some_and(|s| !is_stale(s, now)) => AccountStatus::Fresh,
        None => AccountStatus::Stale,
    }
}

#[must_use]
pub fn is_stale(snapshot: &SnapshotEntry, now: Timestamp) -> bool {
    now.duration_since(snapshot.data_time()) > STALE_AFTER
}

#[must_use]
pub fn source(snapshot: &SnapshotEntry) -> DataSource {
    match (snapshot.origin, snapshot.snapshot.source) {
        (SnapshotOrigin::Cache, _) => DataSource::Cache,
        (SnapshotOrigin::Refreshed, LimitsSource::Live) => DataSource::Live,
        (SnapshotOrigin::Refreshed, LimitsSource::LocalLog { .. }) => DataSource::LocalLog,
    }
}

#[must_use]
pub fn error_view(failure: &RefreshFailure) -> AccountError {
    AccountError {
        kind: error_kind(failure).to_owned(),
        message: failure.to_string(),
    }
}

fn error_kind(failure: &RefreshFailure) -> &'static str {
    match failure {
        RefreshFailure::Timeout => "timeout",
        RefreshFailure::NoProvider => "no_provider",
        RefreshFailure::Provider(error) => match error {
            ProviderError::NotSignedIn => "not_signed_in",
            ProviderError::SignInExpired => "sign_in_expired",
            ProviderError::AccountChanged(_) => "account_changed",
            ProviderError::ApiKeyOnly => "api_key_only",
            ProviderError::NoSubscription { .. } => "no_subscription",
            ProviderError::RateLimited { .. } => "rate_limited",
            ProviderError::Network(_) => "network",
            ProviderError::InvalidResponse(_) => "invalid_response",
            ProviderError::LocalData(_) => "local_data",
            ProviderError::Unsupported(_) => "unsupported",
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{snapshot, ts};

    fn entry(fetched_at: &str, origin: SnapshotOrigin) -> SnapshotEntry {
        SnapshotEntry {
            snapshot: snapshot(Vec::new(), fetched_at),
            origin,
        }
    }

    fn failing(failure: RefreshFailure) -> AccountRuntime {
        AccountRuntime {
            failure: Some(failure),
            ..AccountRuntime::default()
        }
    }

    const NOW: &str = "2026-09-23T10:00:00Z";

    #[test]
    fn snapshot_older_than_ten_minutes_is_stale() {
        let now = ts(NOW);
        let fresh = entry("2026-09-23T09:50:00Z", SnapshotOrigin::Refreshed);
        let stale = entry("2026-09-23T09:49:59Z", SnapshotOrigin::Refreshed);
        assert_eq!(status(None, Some(&fresh), now), AccountStatus::Fresh);
        assert_eq!(status(None, Some(&stale), now), AccountStatus::Stale);
        assert_eq!(status(None, None, now), AccountStatus::Stale);
    }

    #[test]
    fn local_log_staleness_uses_observation_time() {
        let mut local = entry(NOW, SnapshotOrigin::Refreshed);
        local.snapshot.source = LimitsSource::LocalLog {
            observed_at: ts("2026-09-23T09:00:00Z"),
        };
        assert_eq!(status(None, Some(&local), ts(NOW)), AccountStatus::Stale);
        assert_eq!(source(&local), DataSource::LocalLog);
    }

    #[test]
    fn refreshing_wins_then_signed_out_then_error() {
        let now = ts(NOW);
        let fresh = entry(NOW, SnapshotOrigin::Refreshed);
        let mut refreshing = failing(RefreshFailure::Timeout);
        refreshing.refreshing = true;
        let expired = failing(RefreshFailure::Provider(ProviderError::SignInExpired));
        let network = failing(RefreshFailure::Provider(ProviderError::Network("x".into())));
        assert_eq!(
            status(Some(&refreshing), Some(&fresh), now),
            AccountStatus::Refreshing
        );
        assert_eq!(
            status(Some(&expired), Some(&fresh), now),
            AccountStatus::SignedOut
        );
        assert_eq!(
            status(Some(&network), Some(&fresh), now),
            AccountStatus::Error
        );
    }

    #[test]
    fn no_subscription_ranks_below_refreshing_only() {
        let now = ts(NOW);
        let fresh = entry(NOW, SnapshotOrigin::Refreshed);
        let mut lapsed = failing(RefreshFailure::Provider(ProviderError::NoSubscription {
            detail: "No active Claude subscription.".into(),
        }));
        assert_eq!(
            status(Some(&lapsed), Some(&fresh), now),
            AccountStatus::NoSubscription
        );
        assert_eq!(
            status(Some(&lapsed), None, now),
            AccountStatus::NoSubscription
        );
        let view = error_view(lapsed.failure.as_ref().unwrap());
        assert_eq!(view.kind, "no_subscription");
        assert_eq!(view.message, "No active Claude subscription.");
        lapsed.refreshing = true;
        assert_eq!(
            status(Some(&lapsed), Some(&fresh), now),
            AccountStatus::Refreshing
        );
    }

    #[test]
    fn cached_snapshots_report_cache_source() {
        assert_eq!(
            source(&entry(NOW, SnapshotOrigin::Cache)),
            DataSource::Cache
        );
        assert_eq!(
            source(&entry(NOW, SnapshotOrigin::Refreshed)),
            DataSource::Live
        );
    }

    #[test]
    fn errors_carry_kind_and_safe_message() {
        let view = error_view(&RefreshFailure::Provider(ProviderError::RateLimited {
            retry_after: None,
        }));
        assert_eq!(view.kind, "rate_limited");
        assert_eq!(view.message, "rate limited by the provider");
        assert_eq!(error_view(&RefreshFailure::Timeout).kind, "timeout");
    }
}
