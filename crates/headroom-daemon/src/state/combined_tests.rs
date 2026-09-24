use headroom_core::pace::{Severity, Tone};

use super::*;
use crate::state::test_views::{account, tracked, window};
use crate::testing::{CLAUDE, CODEX, ts};

const EARLY: &str = "2026-09-23T12:00:00Z";
const LATE: &str = "2026-09-26T10:00:00Z";

fn codex(name: &str, used: f64) -> AccountView {
    account(
        &CODEX,
        name,
        vec![window("session", used, LATE), window("weekly", used, LATE)],
    )
}

fn ids(groups: &[CombinedView]) -> Vec<Vec<&str>> {
    groups
        .iter()
        .map(|g| g.account_ids.iter().map(String::as_str).collect())
        .collect()
}

#[test]
fn nothing_is_combined_when_the_setting_is_off() {
    let accounts = [codex("a", 10.0), codex("b", 20.0)];
    assert!(combined(&accounts, false).is_empty());
    assert_eq!(ids(&combined(&accounts, true)), [["codex:a", "codex:b"]]);
}

#[test]
fn only_visible_signed_in_accounts_with_windows_are_grouped() {
    let exclude = |edit: fn(&mut AccountView)| {
        let mut left_out = codex("x", 50.0);
        edit(&mut left_out);
        left_out
    };
    let cases: [(&str, AccountView, bool); 8] = [
        ("fresh", codex("x", 50.0), true),
        ("stale", exclude(|a| a.status = AccountStatus::Stale), true),
        (
            "refreshing",
            exclude(|a| a.status = AccountStatus::Refreshing),
            true,
        ),
        (
            "error with windows",
            exclude(|a| a.status = AccountStatus::Error),
            true,
        ),
        ("hidden", exclude(|a| a.hidden = true), false),
        (
            "signed out",
            exclude(|a| a.status = AccountStatus::SignedOut),
            false,
        ),
        (
            "no subscription",
            exclude(|a| a.status = AccountStatus::NoSubscription),
            false,
        ),
        (
            "error without windows",
            exclude(|a| {
                a.status = AccountStatus::Error;
                a.windows.clear();
            }),
            false,
        ),
    ];
    for (name, candidate, grouped) in cases {
        let accounts = [codex("a", 10.0), candidate];
        let groups = combined(&accounts, true);
        assert_eq!(groups.len(), usize::from(grouped), "{name}");
        assert_eq!(is_combinable(&accounts[1]), grouped, "{name}");
    }
}

#[test]
fn an_account_whose_windows_are_all_hidden_stays_alone() {
    let mut all_hidden = codex("b", 20.0);
    for w in &mut all_hidden.windows {
        w.hidden = true;
    }
    assert!(combined(&[codex("a", 10.0), all_hidden], true).is_empty());
}

#[test]
fn single_accounts_and_other_providers_form_no_group() {
    let claude = account(&CLAUDE, "main", vec![window("session", 40.0, EARLY)]);
    let accounts = [codex("a", 10.0), claude.clone(), codex("b", 20.0)];
    let groups = combined(&accounts, true);
    assert_eq!(ids(&groups), [["codex:a", "codex:b"]]);
    assert_eq!(groups[0].provider, CODEX);
    assert_eq!(groups[0].provider_name, "Codex");
    let two_claudes = [claude.clone(), account(&CLAUDE, "work", claude.windows)];
    let groups = combined(
        &[
            codex("a", 10.0),
            two_claudes[0].clone(),
            two_claudes[1].clone(),
        ],
        true,
    );
    assert_eq!(ids(&groups), [["claude:main", "claude:work"]]);
}

#[test]
fn accounts_list_labels_and_plans() {
    let mut b = codex("b", 20.0);
    b.label = None;
    b.email = Some("b@example.com".into());
    b.plan = Some("Plus".into());
    let group = &combined(&[codex("a", 10.0), b], true)[0];
    let entries: Vec<_> = group
        .accounts
        .iter()
        .map(|a| (a.account_id.as_str(), a.label.as_str(), a.plan.as_deref()))
        .collect();
    assert_eq!(
        entries,
        [
            ("codex:a", "a", Some("Pro")),
            ("codex:b", "b@example.com", Some("Plus"))
        ]
    );
}

#[test]
fn windows_match_by_id_and_partial_windows_count_only_their_accounts() {
    let a = account(
        &CODEX,
        "a",
        vec![window("session", 10.0, EARLY), window("weekly", 30.0, LATE)],
    );
    let b = account(
        &CODEX,
        "b",
        vec![
            window("weekly", 50.0, EARLY),
            window("model:spark", 80.0, LATE),
            window("session", 5.0, LATE),
        ],
    );
    let group = &combined(&[a, b], true)[0];
    let shape: Vec<_> = group
        .windows
        .iter()
        .map(|w| (w.id.as_str(), w.capacity_percent, w.segments.len()))
        .collect();
    assert_eq!(
        shape,
        [
            ("session", 200, 2),
            ("weekly", 200, 2),
            ("model:spark", 100, 1)
        ]
    );
    let spark = &group.windows[2];
    assert_eq!(spark.segments[0].account_id, "codex:b");
    assert_eq!((spark.used_percent, spark.remaining_percent), (80.0, 20.0));
}

#[test]
fn hidden_windows_are_left_out_per_account() {
    let mut b = codex("b", 20.0);
    b.windows[1].hidden = true;
    let group = &combined(&[codex("a", 10.0), b], true)[0];
    let weekly = &group.windows[1];
    assert_eq!(weekly.id, "weekly");
    assert_eq!(weekly.capacity_percent, 100);
    assert_eq!(weekly.segments[0].account_id, "codex:a");
    assert_eq!(group.windows[0].capacity_percent, 200);
}

#[test]
fn combined_window_sums_segments_and_takes_the_earliest_reset() {
    let mut a = account(&CODEX, "work", vec![window("weekly", 5.0, LATE)]);
    a.windows[0].tone = Tone::Good;
    let mut b = account(&CODEX, "personal", vec![window("weekly", 50.0, EARLY)]);
    b.windows[0].tone = Tone::Warning;
    let group = &combined(&[a, b], true)[0];
    let weekly = &group.windows[0];
    assert_eq!(weekly.label, "Weekly");
    assert_eq!(weekly.capacity_percent, 200);
    assert_eq!(
        (weekly.used_percent, weekly.remaining_percent),
        (55.0, 145.0)
    );
    assert_eq!(weekly.resets_at, Some(ts(EARLY)));
    let segments: Vec<_> = weekly
        .segments
        .iter()
        .map(|s| {
            (
                s.label.as_str(),
                s.used_percent,
                s.remaining_percent,
                s.resets_at,
                s.tone,
            )
        })
        .collect();
    assert_eq!(
        segments,
        [
            ("work", 5.0, 95.0, Some(ts(LATE)), Tone::Good),
            ("personal", 50.0, 50.0, Some(ts(EARLY)), Tone::Warning)
        ]
    );
}

#[test]
fn raw_segment_values_are_kept_exact() {
    let a = account(&CODEX, "a", vec![window("weekly", 33.333_333, LATE)]);
    let b = account(&CODEX, "b", vec![window("weekly", 101.5, LATE)]);
    let weekly = &combined(&[a, b], true)[0].windows[0];
    let bits = |values: [f64; 4]| values.map(f64::to_bits);
    assert_eq!(
        bits([
            weekly.segments[0].used_percent,
            weekly.segments[1].used_percent,
            weekly.segments[1].remaining_percent,
            weekly.used_percent,
        ]),
        bits([33.333_333, 101.5, 0.0, 33.333_333 + 101.5])
    );
}

#[test]
fn combined_window_carries_the_combined_pace_and_tone() {
    let a = account(
        &CODEX,
        "a",
        vec![tracked(
            window("weekly", 60.0, LATE),
            Severity::RunningOut,
            150.0,
        )],
    );
    let b = account(
        &CODEX,
        "b",
        vec![tracked(
            window("weekly", 40.0, LATE),
            Severity::Healthy,
            80.0,
        )],
    );
    let weekly = &combined(&[a, b], true)[0].windows[0];
    assert_eq!(weekly.pace.severity, Severity::RunningOut);
    assert_eq!(weekly.pace.projected_percent, Some(230.0));
    assert_eq!(weekly.pace.even_pace_percent, Some(100.0));
    assert_eq!(weekly.tone, Tone::Warning);
}
