use headroom_core::account::CredentialOwner;
use headroom_core::pace::{Severity, Tone};

use super::*;
use crate::state::payload::{AccountStatus, CombinedView, PaceView};
use crate::testing::{CLAUDE, CODEX};

fn view(id: &str, hidden: bool, windows: Vec<WindowView>) -> AccountView {
    AccountView {
        id: id.into(),
        provider: CODEX,
        provider_name: "Codex".into(),
        label: None,
        email: None,
        plan: None,
        hidden,
        owner: CredentialOwner::Cli,
        status: AccountStatus::Fresh,
        error: None,
        recovery: None,
        updated_at: None,
        source: None,
        windows,
        balances: Vec::new(),
        notices: Vec::new(),
        usage_home: "~/.codex".into(),
    }
}

fn window(id: &str, remaining: f64, tone: Tone) -> WindowView {
    WindowView {
        id: id.into(),
        label: id.into(),
        used_percent: 100.0 - remaining,
        remaining_percent: remaining,
        resets_at: None,
        period_seconds: None,
        tone,
        pace: PaceView {
            severity: Severity::Untracked,
            even_pace_percent: None,
            projected_percent: None,
            spare_percent: None,
            runs_out_at: None,
        },
        hidden: false,
    }
}

#[test]
fn headline_prefers_highest_tone_then_lowest_remaining() {
    let accounts = [
        view(
            "a",
            false,
            vec![
                window("session", 5.0, Tone::Good),
                window("weekly", 40.0, Tone::Warning),
            ],
        ),
        view("b", false, vec![window("session", 30.0, Tone::Warning)]),
        view("c", true, vec![window("session", 1.0, Tone::Critical)]),
    ];
    let chosen = headline(&accounts, &[], &HeadlineMode::Auto {}).unwrap();
    assert_eq!(
        (chosen.account_id.as_str(), chosen.window.as_str()),
        ("b", "session")
    );
    assert_eq!(chosen.tone, Tone::Warning);
    assert!((chosen.remaining_percent - 30.0).abs() < f64::EPSILON);
}

#[test]
fn headline_ties_keep_account_order() {
    let accounts = [
        view("a", false, vec![window("session", 50.0, Tone::Good)]),
        view("b", false, vec![window("session", 50.0, Tone::Good)]),
    ];
    let chosen = headline(&accounts, &[], &HeadlineMode::Auto {}).unwrap();
    assert_eq!(chosen.account_id, "a");
}

#[test]
fn pinned_headline_falls_back_to_auto_when_missing() {
    let accounts = [
        view("a", false, vec![window("session", 90.0, Tone::Good)]),
        view("b", false, vec![window("weekly", 10.0, Tone::Critical)]),
    ];
    let pin = |account: &str, window: &str| HeadlineMode::Pinned {
        account_id: account.into(),
        window: window.into(),
    };
    let chosen = headline(&accounts, &[], &pin("a", "session")).unwrap();
    assert_eq!(chosen.account_id, "a");
    let fallback = headline(&accounts, &[], &pin("a", "weekly")).unwrap();
    assert_eq!(fallback.account_id, "b");
    assert!(headline(&[], &[], &HeadlineMode::Auto {}).is_none());
}

#[test]
fn hidden_windows_are_skipped() {
    let mut accounts = [
        view("a", false, vec![window("session", 50.0, Tone::Good)]),
        view("b", false, vec![window("weekly", 5.0, Tone::Critical)]),
    ];
    accounts[1].windows[0].hidden = true;
    let chosen = headline(&accounts, &[], &HeadlineMode::Auto {}).unwrap();
    assert_eq!(chosen.account_id, "a");
    let pin = HeadlineMode::Pinned {
        account_id: "b".into(),
        window: "weekly".into(),
    };
    assert_eq!(headline(&accounts, &[], &pin).unwrap().account_id, "a");
    accounts[0].windows[0].hidden = true;
    assert!(headline(&accounts, &[], &HeadlineMode::Auto {}).is_none());
}

#[test]
fn headline_names_the_account_and_window() {
    let mut labelled = view("a", false, vec![window("session", 40.0, Tone::Good)]);
    labelled.label = Some("Work".into());
    labelled.email = Some("ada@example.com".into());
    let mut emailed = labelled.clone();
    emailed.label = None;
    let mut anonymous = emailed.clone();
    anonymous.email = None;
    anonymous.provider = CLAUDE;
    anonymous.provider_name = "Claude".into();
    for (account, expected) in [
        (labelled, "Work"),
        (emailed, "ada@example.com"),
        (anonymous, "Claude"),
    ] {
        let chosen = headline(&[account], &[], &HeadlineMode::Auto {}).unwrap();
        assert_eq!(chosen.account_label.as_deref(), Some(expected));
        assert!(!chosen.combined);
        assert_eq!(chosen.account_count, 1);
        assert_eq!(chosen.window_label, "session");
        assert!((chosen.used_percent - 60.0).abs() < f64::EPSILON);
    }
}

fn grouped(accounts: &[AccountView]) -> Vec<CombinedView> {
    crate::state::combined::combined(accounts, true)
}

fn codex_pair() -> [AccountView; 2] {
    [
        view(
            "codex:a",
            false,
            vec![window("session", 10.0, Tone::Critical)],
        ),
        view("codex:b", false, vec![window("session", 90.0, Tone::Good)]),
    ]
}

fn claude_main(tone: Tone) -> AccountView {
    let mut claude = view("claude:main", false, vec![window("session", 60.0, tone)]);
    claude.provider = CLAUDE;
    claude.provider_name = "Claude".into();
    claude
}

fn pin(account: &str, window: &str) -> HeadlineMode {
    HeadlineMode::Pinned {
        account_id: account.into(),
        window: window.into(),
    }
}

#[test]
fn combined_windows_replace_their_accounts_as_headline_candidates() {
    let accounts = codex_pair();
    let single = headline(&accounts, &[], &HeadlineMode::Auto {}).unwrap();
    assert_eq!(single.account_id, "codex:a");
    assert!(!single.combined);
    let chosen = headline(&accounts, &grouped(&accounts), &HeadlineMode::Auto {}).unwrap();
    assert!(chosen.combined);
    assert_eq!(chosen.account_count, 2);
    assert_eq!(chosen.account_label, None);
    assert_eq!(chosen.account_id, "codex:a");
    assert_eq!(chosen.provider, CODEX);
    assert_eq!(
        (chosen.window.as_str(), chosen.window_label.as_str()),
        ("session", "session")
    );
    assert!((chosen.remaining_percent - 50.0).abs() < f64::EPSILON);
    assert!((chosen.used_percent - 50.0).abs() < f64::EPSILON);
    assert_eq!(chosen.tone, Tone::Good);
}

#[test]
fn combined_and_single_candidates_compete_on_tone_then_share_left() {
    let [a, b] = codex_pair();
    let accounts = [claude_main(Tone::Good), a.clone(), b.clone()];
    let chosen = headline(&accounts, &grouped(&accounts), &HeadlineMode::Auto {}).unwrap();
    assert!(chosen.combined);
    let accounts = [claude_main(Tone::Warning), a, b];
    let chosen = headline(&accounts, &grouped(&accounts), &HeadlineMode::Auto {}).unwrap();
    assert_eq!(chosen.account_id, "claude:main");
    assert!(!chosen.combined);
    assert_eq!(chosen.account_label.as_deref(), Some("Claude"));
}

#[test]
fn a_pin_on_a_grouped_account_resolves_to_its_combined_window() {
    let accounts = codex_pair();
    let groups = grouped(&accounts);
    for account in ["codex:a", "codex:b"] {
        let chosen = headline(&accounts, &groups, &pin(account, "session")).unwrap();
        assert!(chosen.combined, "{account}");
        assert_eq!(chosen.account_count, 2);
    }
    let with_claude = [
        claude_main(Tone::Good),
        accounts[0].clone(),
        accounts[1].clone(),
    ];
    let groups = grouped(&with_claude);
    let fallback = headline(&with_claude, &groups, &pin("codex:b", "weekly")).unwrap();
    assert!(fallback.combined);
    let pinned = headline(&with_claude, &groups, &pin("claude:main", "session")).unwrap();
    assert_eq!(pinned.account_id, "claude:main");
    let off = headline(&accounts, &[], &pin("codex:b", "session")).unwrap();
    assert_eq!(off.account_id, "codex:b");
    assert!(!off.combined);
}

#[test]
fn accounts_left_out_of_a_group_keep_their_own_candidates() {
    let [a, b] = codex_pair();
    let mut signed_out = view(
        "codex:c",
        false,
        vec![window("session", 1.0, Tone::Critical)],
    );
    signed_out.status = AccountStatus::SignedOut;
    let hidden = view(
        "codex:d",
        true,
        vec![window("session", 0.0, Tone::Critical)],
    );
    let accounts = [a, b, signed_out, hidden];
    let groups = grouped(&accounts);
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].account_ids, ["codex:a", "codex:b"]);
    let chosen = headline(&accounts, &groups, &HeadlineMode::Auto {}).unwrap();
    assert_eq!(chosen.account_id, "codex:c");
    assert!(!chosen.combined);
}
