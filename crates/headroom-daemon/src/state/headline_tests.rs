use headroom_core::account::{CredentialOwner, ProviderKind};
use headroom_core::pace::{Severity, Tone};

use super::*;
use crate::state::payload::{AccountStatus, PaceView};

fn view(id: &str, hidden: bool, windows: Vec<WindowView>) -> AccountView {
    AccountView {
        id: id.into(),
        provider: ProviderKind::Codex,
        label: None,
        email: None,
        plan: None,
        hidden,
        owner: CredentialOwner::Cli,
        status: AccountStatus::Fresh,
        error: None,
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
    let chosen = headline(&accounts, &HeadlineMode::Auto {}).unwrap();
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
    let chosen = headline(&accounts, &HeadlineMode::Auto {}).unwrap();
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
    let chosen = headline(&accounts, &pin("a", "session")).unwrap();
    assert_eq!(chosen.account_id, "a");
    let fallback = headline(&accounts, &pin("a", "weekly")).unwrap();
    assert_eq!(fallback.account_id, "b");
    assert!(headline(&[], &HeadlineMode::Auto {}).is_none());
}

#[test]
fn hidden_windows_are_skipped() {
    let mut accounts = [
        view("a", false, vec![window("session", 50.0, Tone::Good)]),
        view("b", false, vec![window("weekly", 5.0, Tone::Critical)]),
    ];
    accounts[1].windows[0].hidden = true;
    let chosen = headline(&accounts, &HeadlineMode::Auto {}).unwrap();
    assert_eq!(chosen.account_id, "a");
    let pin = HeadlineMode::Pinned {
        account_id: "b".into(),
        window: "weekly".into(),
    };
    assert_eq!(headline(&accounts, &pin).unwrap().account_id, "a");
    accounts[0].windows[0].hidden = true;
    assert!(headline(&accounts, &HeadlineMode::Auto {}).is_none());
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
    anonymous.provider = ProviderKind::Claude;
    for (account, expected) in [
        (labelled, "Work"),
        (emailed, "ada@example.com"),
        (anonymous, "Claude Code"),
    ] {
        let chosen = headline(&[account], &HeadlineMode::Auto {}).unwrap();
        assert_eq!(chosen.account_label, expected);
        assert_eq!(chosen.window_label, "session");
        assert!((chosen.used_percent - 60.0).abs() < f64::EPSILON);
    }
}
