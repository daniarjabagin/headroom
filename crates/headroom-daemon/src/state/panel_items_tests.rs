use headroom_core::pace::{Severity, Tone};

use super::*;
use crate::settings::HeadlineMode;
use crate::state::combined::combined;
use crate::state::payload::{AccountStatus, WindowView};
use crate::state::test_views::{account, tracked, window};
use crate::testing::{CLAUDE, CODEX};

const LATE: &str = "2026-09-26T10:00:00Z";

type ModeCase = (
    PanelMode,
    Vec<PanelLimit>,
    Vec<(&'static str, &'static str)>,
);

fn toned(id: &str, used: f64, tone: Tone) -> WindowView {
    WindowView {
        tone,
        ..window(id, used, LATE)
    }
}

fn sample_accounts() -> Vec<AccountView> {
    vec![
        account(
            &CODEX,
            "work",
            vec![
                toned("session", 30.0, Tone::Good),
                toned("weekly", 60.0, Tone::Warning),
            ],
        ),
        account(&CLAUDE, "main", vec![toned("session", 85.0, Tone::Warning)]),
        account(&CLAUDE, "spare", vec![toned("session", 10.0, Tone::Good)]),
    ]
}

fn limit(account_id: &str, window: &str) -> PanelLimit {
    PanelLimit {
        account_id: account_id.into(),
        window: window.into(),
    }
}

fn settings(mode: PanelMode, limits: Vec<PanelLimit>) -> Settings {
    let mut settings = Settings::default();
    settings.display.panel_mode = mode;
    settings.display.panel_limits = limits;
    settings
}

fn chosen(items: &[PanelItem]) -> Vec<(&str, &str)> {
    items
        .iter()
        .map(|i| (i.headline.account_id.as_str(), i.headline.window.as_str()))
        .collect()
}

#[test]
fn each_mode_resolves_its_items() {
    let cases: [ModeCase; 5] = [
        (
            PanelMode::Headline,
            vec![],
            vec![("claude:main", "session")],
        ),
        (
            PanelMode::Icon,
            vec![limit("codex:work", "session")],
            vec![],
        ),
        (
            PanelMode::Several,
            vec![],
            vec![("claude:main", "session"), ("codex:work", "weekly")],
        ),
        (
            PanelMode::Several,
            vec![
                limit("claude:spare", "session"),
                limit("codex:work", "session"),
            ],
            vec![("claude:spare", "session"), ("codex:work", "session")],
        ),
        (
            PanelMode::Several,
            vec![limit("codex:gone", "session"), limit("codex:work", "opus")],
            vec![],
        ),
    ];
    let accounts = sample_accounts();
    for (mode, limits, expected) in cases {
        let items = panel_items(&accounts, &[], &settings(mode, limits.clone()));
        assert_eq!(chosen(&items), expected, "{mode:?} {limits:?}");
    }
}

#[test]
fn headline_mode_follows_a_pinned_headline() {
    let mut settings = settings(PanelMode::Headline, vec![]);
    settings.headline = HeadlineMode::Pinned {
        account_id: "claude:spare".into(),
        window: "session".into(),
    };
    let items = panel_items(&sample_accounts(), &[], &settings);
    assert_eq!(chosen(&items), [("claude:spare", "session")]);
}

#[test]
fn pins_skip_hidden_accounts_hidden_windows_and_lapsed_accounts() {
    let mut accounts = sample_accounts();
    accounts[0].windows[1].hidden = true;
    accounts[1].hidden = true;
    accounts[2].status = AccountStatus::NoSubscription;
    let limits = vec![
        limit("codex:work", "weekly"),
        limit("claude:main", "session"),
        limit("claude:spare", "session"),
        limit("codex:work", "session"),
    ];
    let items = panel_items(&accounts, &[], &settings(PanelMode::Several, limits));
    assert_eq!(chosen(&items), [("codex:work", "session")]);
}

#[test]
fn several_never_shows_more_than_three_items() {
    let accounts = sample_accounts();
    let limits = vec![
        limit("codex:work", "session"),
        limit("codex:work", "weekly"),
        limit("claude:main", "session"),
        limit("claude:spare", "session"),
    ];
    let items = panel_items(&accounts, &[], &settings(PanelMode::Several, limits));
    assert_eq!(items.len(), 3);
    let auto = panel_items(&accounts, &[], &settings(PanelMode::Several, vec![]));
    assert_eq!(auto.len(), 2);
}

#[test]
fn most_critical_ranks_tone_then_remaining_then_order() {
    let accounts = vec![
        account(&CODEX, "a", vec![toned("session", 50.0, Tone::Good)]),
        account(&CODEX, "b", vec![toned("session", 50.0, Tone::Good)]),
        account(&CODEX, "c", vec![toned("session", 20.0, Tone::Good)]),
        account(&CODEX, "d", vec![toned("session", 95.0, Tone::Good)]),
        account(&CODEX, "e", vec![toned("session", 5.0, Tone::Critical)]),
    ];
    let items = panel_items(&accounts, &[], &settings(PanelMode::Several, vec![]));
    assert_eq!(
        chosen(&items),
        [("codex:e", "session"), ("codex:d", "session")]
    );
    let ties = panel_items(&accounts[..2], &[], &settings(PanelMode::Several, vec![]));
    assert_eq!(
        chosen(&ties),
        [("codex:a", "session"), ("codex:b", "session")]
    );
}

#[test]
fn pins_inside_a_combined_group_map_to_the_combined_window_once() {
    let accounts = vec![
        account(&CODEX, "work", vec![toned("session", 50.0, Tone::Good)]),
        account(&CODEX, "home", vec![toned("session", 30.0, Tone::Good)]),
        account(&CLAUDE, "main", vec![toned("session", 10.0, Tone::Good)]),
    ];
    let groups = combined(&accounts, true);
    let limits = vec![
        limit("codex:home", "session"),
        limit("codex:work", "session"),
        limit("claude:main", "session"),
    ];
    let items = panel_items(&accounts, &groups, &settings(PanelMode::Several, limits));
    assert_eq!(items.len(), 2);
    let group = &items[0];
    assert!(group.headline.combined);
    assert_eq!(group.headline.account_label, None);
    assert_eq!(group.headline.account_count, 2);
    assert!((group.value_percent - 60.0).abs() < 1e-9);
    assert_eq!(items[1].headline.account_id, "claude:main");
}

#[test]
fn value_percent_follows_value_mode() {
    let accounts = sample_accounts();
    let mut settings = settings(PanelMode::Several, vec![limit("claude:main", "session")]);
    let left = panel_items(&accounts, &[], &settings);
    assert!((left[0].value_percent - 15.0).abs() < 1e-9);
    settings.display.value_mode = ValueMode::Used;
    let used = panel_items(&accounts, &[], &settings);
    assert!((used[0].value_percent - 85.0).abs() < 1e-9);
    assert_eq!(used[0].logo, "claude");
    assert_eq!(used[0].headline.tone, Tone::Warning);
}

#[test]
fn even_pace_is_a_zero_to_hundred_share() {
    let pace = |id| tracked(window(id, 40.0, LATE), Severity::Healthy, 80.0);
    let accounts = vec![
        account(&CODEX, "work", vec![pace("session")]),
        account(&CODEX, "home", vec![pace("session")]),
        account(&CLAUDE, "main", vec![pace("session")]),
    ];
    let groups = combined(&accounts, true);
    let limits = vec![
        limit("codex:work", "session"),
        limit("claude:main", "session"),
    ];
    let items = panel_items(&accounts, &groups, &settings(PanelMode::Several, limits));
    let paces: Vec<_> = items.iter().map(|i| i.even_pace_percent).collect();
    assert_eq!(paces, [Some(50.0), Some(50.0)]);
    let untracked = sample_accounts();
    let plain = panel_items(&untracked, &[], &settings(PanelMode::Headline, vec![]));
    assert_eq!(plain[0].even_pace_percent, None);
}

#[test]
fn panel_tone_is_the_worst_visible_tone() {
    let mut accounts = sample_accounts();
    assert_eq!(panel_tone(&accounts, &[]), Some(Tone::Warning));
    accounts.push(account(
        &CODEX,
        "hidden",
        vec![toned("session", 99.0, Tone::Critical)],
    ));
    accounts[3].hidden = true;
    assert_eq!(panel_tone(&accounts, &[]), Some(Tone::Warning));
    accounts[3].hidden = false;
    accounts[3].windows[0].hidden = true;
    assert_eq!(panel_tone(&accounts, &[]), Some(Tone::Warning));
    accounts[3].windows[0].hidden = false;
    assert_eq!(panel_tone(&accounts, &[]), Some(Tone::Critical));
    assert_eq!(panel_tone(&[], &[]), None);
}

#[test]
fn icon_mode_has_no_items_but_keeps_the_tone() {
    let accounts = sample_accounts();
    let items = panel_items(&accounts, &[], &settings(PanelMode::Icon, vec![]));
    assert!(items.is_empty());
    assert_eq!(panel_tone(&accounts, &[]), Some(Tone::Warning));
}
