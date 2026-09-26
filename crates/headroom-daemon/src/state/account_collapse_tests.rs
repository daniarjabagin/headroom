use headroom_core::pace::Tone;

use super::*;
use crate::state::combined::combined;
use crate::state::payload::{AccountError, AccountStatus, WindowView};
use crate::state::test_views::{account, window};
use crate::testing::{CLAUDE, CODEX};

const LATE: &str = "2026-09-26T10:00:00Z";

fn toned(tone: Tone) -> WindowView {
    WindowView {
        tone,
        ..window("session", 40.0, LATE)
    }
}

fn display(collapse: bool, starred: &[&str]) -> DisplaySettings {
    DisplaySettings {
        collapse_unstarred: collapse,
        starred_accounts: starred.iter().map(|id| (*id).to_owned()).collect(),
        ..DisplaySettings::default()
    }
}

fn failure(kind: &str) -> AccountError {
    AccountError {
        kind: kind.into(),
        message: "failed".into(),
    }
}

#[test]
fn every_unstarred_account_collapses_whatever_its_tone() {
    let cases = [
        (false, vec![], Tone::Good, false),
        (true, vec![], Tone::Good, true),
        (true, vec![], Tone::Neutral, true),
        (true, vec![], Tone::Warning, true),
        (true, vec![], Tone::Critical, true),
        (true, vec!["claude:main"], Tone::Good, false),
        (true, vec!["claude:main"], Tone::Critical, false),
        (true, vec!["claude:other"], Tone::Critical, true),
    ];
    for (collapse, starred, tone, expected) in cases {
        let view = account(&CLAUDE, "main", vec![toned(Tone::Good), toned(tone)]);
        let settings = display(collapse, &starred);
        assert_eq!(
            account_collapsed(&view, &settings),
            expected,
            "{collapse} {starred:?} {tone:?}"
        );
    }
}

#[test]
fn unstarred_accounts_collapse_whatever_their_status() {
    let cases = [
        (AccountStatus::Fresh, None),
        (AccountStatus::Refreshing, Some(failure("network"))),
        (AccountStatus::SignedOut, Some(failure("not_signed_in"))),
        (AccountStatus::Error, Some(failure("account_changed"))),
        (
            AccountStatus::NoSubscription,
            Some(failure("no_subscription")),
        ),
        (AccountStatus::Fresh, Some(failure("rate_limited"))),
    ];
    for (status, error) in cases {
        let mut view = account(&CLAUDE, "main", vec![toned(Tone::Critical)]);
        view.status = status;
        view.error = error.clone();
        assert!(
            account_collapsed(&view, &display(true, &[])),
            "{status:?} {error:?}"
        );
        assert!(!account_collapsed(&view, &display(true, &["claude:main"])));
        assert!(!account_collapsed(&view, &display(false, &[])));
    }
}

#[test]
fn accounts_without_windows_collapse_when_unstarred() {
    let view = account(&CLAUDE, "main", vec![]);
    assert!(account_collapsed(&view, &display(true, &[])));
}

#[test]
fn combined_groups_collapse_unless_a_member_is_starred() {
    let mut accounts = [
        account(&CODEX, "work", vec![toned(Tone::Good)]),
        account(&CODEX, "home", vec![toned(Tone::Good)]),
    ];
    accounts[1].status = AccountStatus::Error;
    accounts[1].error = Some(failure("network"));
    let cases = [
        (false, vec![], false),
        (true, vec![], true),
        (true, vec!["codex:home"], false),
        (true, vec!["claude:main"], true),
    ];
    for (collapse, starred, expected) in cases {
        let mut groups = combined(&accounts, true);
        groups[0].windows[0].tone = Tone::Critical;
        let groups = with_group_collapse(groups, &display(collapse, &starred));
        assert_eq!(groups[0].account_ids.len(), 2);
        assert_eq!(groups[0].collapsed, expected, "{collapse} {starred:?}");
        assert_eq!(groups[0].windows[0].tone, Tone::Critical);
    }
}
