use jiff::{SignedDuration, Timestamp};

use crate::scheduler::policy::jittered;

pub const CHECK_EVERY: SignedDuration = SignedDuration::from_hours(24);
pub const STARTUP_DELAY: SignedDuration = SignedDuration::from_mins(2);
pub const RETRY_FAILED: SignedDuration = SignedDuration::from_hours(1);
pub const RATE_LIMIT_MIN: SignedDuration = SignedDuration::from_hours(1);

#[must_use]
pub fn first_delay(last_check: Option<Timestamp>, now: Timestamp, sample: f64) -> SignedDuration {
    let every = jittered(CHECK_EVERY, sample);
    let due = last_check.map_or(SignedDuration::ZERO, |at| every - now.duration_since(at));
    due.min(every).max(STARTUP_DELAY)
}

#[must_use]
pub fn after_check(sample: f64) -> SignedDuration {
    jittered(CHECK_EVERY, sample)
}

#[must_use]
pub fn after_rate_limit(retry_after: Option<SignedDuration>) -> SignedDuration {
    retry_after
        .unwrap_or(RATE_LIMIT_MIN)
        .clamp(RATE_LIMIT_MIN, CHECK_EVERY)
}

#[must_use]
pub fn after_failure(sample: f64) -> SignedDuration {
    jittered(RETRY_FAILED, sample)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ts(text: &str) -> Timestamp {
        text.parse().unwrap()
    }

    const NOW: &str = "2026-09-23T10:00:00Z";

    #[test]
    fn the_first_check_waits_for_start_up_or_the_day_since_the_last_one() {
        let now = ts(NOW);
        let cases = [
            (None, STARTUP_DELAY),
            (Some("2026-09-23T09:00:00Z"), SignedDuration::from_hours(23)),
            (Some("2026-09-22T10:01:00Z"), STARTUP_DELAY),
            (Some("2026-09-20T10:00:00Z"), STARTUP_DELAY),
            (Some("2026-09-23T11:00:00Z"), CHECK_EVERY),
        ];
        for (last, expected) in cases {
            assert_eq!(first_delay(last.map(ts), now, 0.5), expected, "{last:?}");
        }
    }

    #[test]
    fn the_daily_check_is_jittered_by_ten_percent() {
        assert_eq!(after_check(0.5), CHECK_EVERY);
        assert_eq!(after_check(0.0), SignedDuration::from_mins(24 * 54));
        assert_eq!(after_check(1.0), SignedDuration::from_mins(24 * 66));
        let late = first_delay(Some(ts("2026-09-23T09:00:00Z")), ts(NOW), 1.0);
        assert_eq!(late, SignedDuration::from_mins(24 * 66 - 60));
    }

    #[test]
    fn rate_limits_wait_at_least_an_hour_and_at_most_a_day() {
        assert_eq!(after_rate_limit(None), RATE_LIMIT_MIN);
        assert_eq!(
            after_rate_limit(Some(SignedDuration::from_secs(30))),
            RATE_LIMIT_MIN
        );
        let reset = SignedDuration::from_mins(90);
        assert_eq!(after_rate_limit(Some(reset)), reset);
        assert_eq!(
            after_rate_limit(Some(SignedDuration::from_hours(48))),
            CHECK_EVERY
        );
    }

    #[test]
    fn failures_retry_in_about_an_hour() {
        assert_eq!(after_failure(0.5), RETRY_FAILED);
        assert_eq!(after_failure(0.0), SignedDuration::from_mins(54));
    }
}
