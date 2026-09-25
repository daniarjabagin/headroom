use jiff::{SignedDuration, Timestamp};

use crate::scheduler::policy::jittered;

pub const POLL_EVERY: SignedDuration = SignedDuration::from_mins(5);
pub const STARTUP_DELAY: SignedDuration = SignedDuration::from_secs(15);
pub const BACKOFF_CAP: SignedDuration = SignedDuration::from_hours(1);
pub const RATE_LIMIT_DEFAULT: SignedDuration = SignedDuration::from_mins(15);
pub const STALE_AFTER: SignedDuration = SignedDuration::from_mins(30);
const MAX_DOUBLINGS: u32 = 8;

#[must_use]
pub fn next_round(sample: f64) -> SignedDuration {
    jittered(POLL_EVERY, sample)
}

/// Waits out the interval since the oldest page a round needs, or starts soon when one is missing.
#[must_use]
pub fn first_round(fetched: &[Option<Timestamp>], now: Timestamp) -> SignedDuration {
    let ages: Option<Vec<SignedDuration>> = fetched
        .iter()
        .map(|at| at.map(|at| now.duration_since(at)))
        .collect();
    match ages.and_then(|ages| ages.into_iter().max()) {
        Some(oldest) => (POLL_EVERY - oldest).clamp(STARTUP_DELAY, POLL_EVERY),
        None => STARTUP_DELAY,
    }
}

#[must_use]
pub fn after_failure(failures: u32, sample: f64) -> SignedDuration {
    let doublings = failures.saturating_sub(1).min(MAX_DOUBLINGS);
    let base = POLL_EVERY
        .checked_mul(1_i32 << doublings)
        .unwrap_or(BACKOFF_CAP)
        .min(BACKOFF_CAP);
    jittered(base, sample).min(BACKOFF_CAP)
}

#[must_use]
pub fn after_rate_limit(retry_after: Option<SignedDuration>) -> SignedDuration {
    retry_after
        .unwrap_or(RATE_LIMIT_DEFAULT)
        .clamp(POLL_EVERY, BACKOFF_CAP)
}

#[must_use]
pub fn is_fresh(fetched_at: Timestamp, now: Timestamp) -> bool {
    now.duration_since(fetched_at) <= STALE_AFTER
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: &str = "2026-09-23T10:00:00Z";

    fn ts(text: &str) -> Timestamp {
        text.parse().unwrap()
    }

    #[test]
    fn rounds_come_every_five_minutes_give_or_take_ten_percent() {
        assert_eq!(next_round(0.5), POLL_EVERY);
        assert_eq!(next_round(0.0), SignedDuration::from_secs(270));
        assert_eq!(next_round(1.0), SignedDuration::from_secs(330));
    }

    #[test]
    fn the_first_round_waits_for_the_oldest_cached_page() {
        let now = ts(NOW);
        let two_min = Some(ts("2026-09-23T09:58:00Z"));
        let one_min = Some(ts("2026-09-23T09:59:00Z"));
        let hour = Some(ts("2026-09-23T09:00:00Z"));
        let future = Some(ts("2026-09-23T11:00:00Z"));
        let cases = [
            (vec![], STARTUP_DELAY),
            (vec![None], STARTUP_DELAY),
            (vec![one_min, None], STARTUP_DELAY),
            (vec![one_min, two_min], SignedDuration::from_mins(3)),
            (vec![hour], STARTUP_DELAY),
            (vec![future], POLL_EVERY),
        ];
        for (fetched, expected) in cases {
            assert_eq!(first_round(&fetched, now), expected, "{fetched:?}");
        }
    }

    #[test]
    fn failures_double_the_wait_up_to_an_hour() {
        assert_eq!(after_failure(1, 0.5), POLL_EVERY);
        assert_eq!(after_failure(2, 0.5), SignedDuration::from_mins(10));
        assert_eq!(after_failure(3, 0.5), SignedDuration::from_mins(20));
        assert_eq!(after_failure(5, 0.5), BACKOFF_CAP);
        assert_eq!(after_failure(u32::MAX, 1.0), BACKOFF_CAP);
    }

    #[test]
    fn rate_limits_wait_between_one_round_and_an_hour() {
        assert_eq!(after_rate_limit(None), RATE_LIMIT_DEFAULT);
        assert_eq!(
            after_rate_limit(Some(SignedDuration::from_secs(10))),
            POLL_EVERY
        );
        assert_eq!(
            after_rate_limit(Some(SignedDuration::from_mins(20))),
            SignedDuration::from_mins(20)
        );
        assert_eq!(
            after_rate_limit(Some(SignedDuration::from_hours(5))),
            BACKOFF_CAP
        );
    }

    #[test]
    fn statuses_go_stale_after_thirty_minutes() {
        let now = ts(NOW);
        assert!(is_fresh(ts("2026-09-23T09:30:00Z"), now));
        assert!(!is_fresh(ts("2026-09-23T09:29:59Z"), now));
        assert!(is_fresh(ts("2026-09-23T10:01:00Z"), now));
    }
}
