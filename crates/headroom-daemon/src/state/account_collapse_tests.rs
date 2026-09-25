use super::*;
use crate::state::combined::combined;
use crate::state::payload::{AccountError, WindowView};
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

#[test]
fn accounts_collapse_only_when_unstarred_and_calm() {
    let cases = [
        (false, vec![], Tone::Good, false),
        (true, vec![], Tone::Good, true),
        (true, vec![], Tone::Neutral, true),
        (true, vec!["claude:main"], Tone::Good, false),
        (true, vec!["claude:other"], Tone::Good, true),
        (true, vec![], Tone::Warning, false),
        (true, vec![], Tone::Critical, false),
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
fn hidden_windows_never_promote_an_account() {
    let mut critical = toned(Tone::Critical);
    critical.hidden = true;
    let view = account(&CLAUDE, "main", vec![toned(Tone::Good), critical]);
    assert!(account_collapsed(&view, &display(true, &[])));
}

fn failure(kind: &str) -> AccountError {
    AccountError {
        kind: kind.into(),
        message: "failed".into(),
    }
}

#[test]
fn accounts_that_need_attention_never_collapse() {
    let cases = [
        (AccountStatus::Fresh, None, true),
        (AccountStatus::Stale, None, true),
        (AccountStatus::Refreshing, None, true),
        (
            AccountStatus::SignedOut,
            Some(failure("not_signed_in")),
            false,
        ),
        (
            AccountStatus::SignedOut,
            Some(failure("sign_in_expired")),
            false,
        ),
        (
            AccountStatus::Error,
            Some(failure("account_changed")),
            false,
        ),
        (AccountStatus::Error, Some(failure("network")), false),
        (AccountStatus::Error, None, false),
        (
            AccountStatus::NoSubscription,
            Some(failure("no_subscription")),
            false,
        ),
        (AccountStatus::Refreshing, Some(failure("network")), false),
        (AccountStatus::Fresh, Some(failure("rate_limited")), false),
    ];
    for (status, error, expected) in cases {
        let mut view = account(&CLAUDE, "main", vec![toned(Tone::Good)]);
        view.status = status;
        view.error = error.clone();
        assert_eq!(
            account_collapsed(&view, &display(true, &[])),
            expected,
            "{status:?} {error:?}"
        );
    }
}

#[test]
fn a_member_that_needs_attention_keeps_the_group_expanded() {
    let mut accounts = [
        account(&CODEX, "work", vec![toned(Tone::Good)]),
        account(&CODEX, "home", vec![toned(Tone::Good)]),
    ];
    accounts[1].status = AccountStatus::Error;
    accounts[1].error = Some(failure("network"));
    let groups = with_group_collapse(combined(&accounts, true), &accounts, &display(true, &[]));
    assert_eq!(groups[0].account_ids.len(), 2);
    assert!(!groups[0].collapsed);
}

#[test]
fn accounts_without_windows_collapse_when_unstarred() {
    let view = account(&CLAUDE, "main", vec![]);
    assert!(account_collapsed(&view, &display(true, &[])));
}

#[test]
fn combined_groups_collapse_by_members_and_combined_tone() {
    let accounts = [
        account(&CODEX, "work", vec![toned(Tone::Good)]),
        account(&CODEX, "home", vec![toned(Tone::Good)]),
    ];
    let cases = [
        (false, vec![], false),
        (true, vec![], true),
        (true, vec!["codex:home"], false),
        (true, vec!["claude:main"], true),
    ];
    for (collapse, starred, expected) in cases {
        let groups = with_group_collapse(
            combined(&accounts, true),
            &accounts,
            &display(collapse, &starred),
        );
        assert_eq!(groups[0].collapsed, expected, "{collapse} {starred:?}");
    }
}

#[test]
fn a_warning_combined_window_promotes_the_group() {
    let accounts = [
        account(&CODEX, "work", vec![toned(Tone::Good)]),
        account(&CODEX, "home", vec![toned(Tone::Good)]),
    ];
    let mut groups = combined(&accounts, true);
    groups[0].windows[0].tone = Tone::Warning;
    let groups = with_group_collapse(groups, &accounts, &display(true, &[]));
    assert!(!groups[0].collapsed);
}
