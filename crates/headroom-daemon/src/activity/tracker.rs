use std::collections::HashMap;

use headroom_core::account::AccountRef;
use jiff::{SignedDuration, Timestamp};

use crate::home::UsageHome;

pub const LIVE_WINDOW: SignedDuration = SignedDuration::from_mins(10);

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ActivityTracker {
    last_writes: HashMap<UsageHome, Timestamp>,
}

impl ActivityTracker {
    pub fn record(&mut self, home: &UsageHome, at: Timestamp, now: Timestamp) -> bool {
        let was_live = self.home_is_live(home, now);
        let last = self.last_writes.entry(home.clone()).or_insert(at);
        *last = (*last).max(at);
        !was_live && self.home_is_live(home, now)
    }

    #[must_use]
    pub fn last_write(&self, home: &UsageHome) -> Option<Timestamp> {
        self.last_writes.get(home).copied()
    }

    #[must_use]
    pub fn home_is_live(&self, home: &UsageHome, now: Timestamp) -> bool {
        self.last_write(home)
            .is_some_and(|at| within_window(at, now))
    }

    #[must_use]
    pub fn account_is_live(&self, account: &AccountRef, now: Timestamp) -> bool {
        self.last_writes.iter().any(|(home, at)| {
            home.provider == account.provider
                && home.home == account.home
                && within_window(*at, now)
        })
    }
}

fn within_window(at: Timestamp, now: Timestamp) -> bool {
    now.duration_since(at) < LIVE_WINDOW
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{CLAUDE, CODEX, account, ts, usage_home_of};

    const NOW: &str = "2026-09-23T10:00:00Z";
    const MINUTE_LATER: &str = "2026-09-23T10:01:00Z";

    #[test]
    fn a_fresh_write_turns_the_home_live_once() {
        let mut tracker = ActivityTracker::default();
        let home = usage_home_of(&account(CODEX, "work"));
        assert!(tracker.record(&home, ts(NOW), ts(NOW)));
        assert!(!tracker.record(&home, ts(MINUTE_LATER), ts(MINUTE_LATER)));
        assert_eq!(tracker.last_write(&home), Some(ts(MINUTE_LATER)));
    }

    #[test]
    fn writes_older_than_the_window_do_not_make_a_home_live() {
        let mut tracker = ActivityTracker::default();
        let home = usage_home_of(&account(CODEX, "work"));
        assert!(!tracker.record(&home, ts("2026-09-23T09:50:00Z"), ts(NOW)));
        assert!(!tracker.home_is_live(&home, ts(NOW)));
        assert!(tracker.home_is_live(&home, ts("2026-09-23T09:59:59Z")));
    }

    #[test]
    fn an_older_write_never_moves_the_last_write_back() {
        let mut tracker = ActivityTracker::default();
        let home = usage_home_of(&account(CODEX, "work"));
        tracker.record(&home, ts(NOW), ts(NOW));
        tracker.record(&home, ts("2026-09-23T09:00:00Z"), ts(NOW));
        assert_eq!(tracker.last_write(&home), Some(ts(NOW)));
    }

    #[test]
    fn a_home_turns_live_again_after_a_quiet_window() {
        let mut tracker = ActivityTracker::default();
        let home = usage_home_of(&account(CODEX, "work"));
        tracker.record(&home, ts(NOW), ts(NOW));
        let later = ts("2026-09-23T10:10:00Z");
        assert!(!tracker.home_is_live(&home, later));
        assert!(tracker.record(&home, later, later));
    }

    #[test]
    fn accounts_are_live_only_through_their_own_home() {
        let mut tracker = ActivityTracker::default();
        let work = account(CODEX, "work");
        let personal = account(CODEX, "personal");
        let claude = account(CLAUDE, "main");
        tracker.record(&usage_home_of(&work), ts(NOW), ts(NOW));
        assert!(tracker.account_is_live(&work, ts(NOW)));
        assert!(tracker.account_is_live(&personal, ts(NOW)));
        assert!(!tracker.account_is_live(&claude, ts(NOW)));
        assert!(!tracker.account_is_live(&work, ts("2026-09-23T10:10:00Z")));
    }
}
