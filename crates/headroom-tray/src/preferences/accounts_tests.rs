use super::*;
use crate::payload::{State, parse_state};
use crate::preferences::registry::ProviderLinks;

const SAMPLE: &str = include_str!("../../../../shell/gnome/dev/sample-state.json");

fn sample() -> State {
    parse_state(SAMPLE).unwrap()
}

fn items(state: &State, display: &Display) -> Vec<SidebarItem> {
    sidebar_items(Lang::En, &state.accounts, display)
}

fn provider(links: ProviderLinks) -> ProviderInfo {
    ProviderInfo {
        id: "claude".into(),
        display_name: "Claude".into(),
        methods: Vec::new(),
        links,
    }
}

#[test]
fn sidebar_keeps_the_daemon_order_and_names_accounts_of_one_provider() {
    let state = sample();
    let titles: Vec<String> = items(&state, &Display::default())
        .into_iter()
        .map(|item| item.title)
        .collect();
    assert_eq!(
        titles,
        [
            "Codex · work",
            "Codex · personal",
            "Claude · personal",
            "Claude · team",
            "OpenRouter",
            "Grok",
        ]
    );
}

#[test]
fn subtitles_show_the_plan_and_an_email_the_title_does_not_carry() {
    let state = sample();
    let subtitles: Vec<String> = items(&state, &Display::default())
        .into_iter()
        .map(|item| item.subtitle)
        .collect();
    assert_eq!(subtitles[0], "Pro · dev@example.com");
    assert_eq!(subtitles[4], "");
    assert_eq!(subtitles[5], "SuperGrok · dev@example.com");
    let mut unlabelled = state.accounts[0].clone();
    unlabelled.label = None;
    let pair = [unlabelled, state.accounts[1].clone()];
    let listed = sidebar_items(Lang::En, &pair, &Display::default());
    assert_eq!(listed[0].title, "Codex · dev@example.com");
    assert_eq!(listed[0].subtitle, "Pro");
}

#[test]
fn a_single_labelled_account_still_shows_its_label() {
    let state = sample();
    let alone = [state.accounts[5].clone()];
    let mut labelled = alone.clone();
    labelled[0].label = Some("home".into());
    assert_eq!(sidebar_title(&alone[0], &alone), "Grok");
    assert_eq!(sidebar_title(&labelled[0], &labelled), "Grok · home");
}

#[test]
fn marks_follow_problems_first_then_the_worst_shown_limit() {
    let state = sample();
    let listed = items(&state, &Display::default());
    let marks: Vec<Mark> = listed.iter().map(|item| item.mark).collect();
    assert_eq!(
        marks,
        [
            Mark::Tone(Tone::Good),
            Mark::Tone(Tone::Critical),
            Mark::Problem,
            Mark::Problem,
            Mark::Tone(Tone::Neutral),
            Mark::Tone(Tone::Good),
        ]
    );
    assert_eq!(listed[1].note, Some("Outdated"));
    assert_eq!(listed[3].note, Some("Signed out"));
    assert_eq!(listed[0].note, None);
}

#[test]
fn hidden_limits_do_not_colour_the_mark() {
    let state = sample();
    let mut display = Display::default();
    display.hidden_windows.insert(
        state.accounts[1].id.clone(),
        vec!["weekly".into(), "model:spark".into()],
    );
    assert_eq!(
        account_mark(&state.accounts[1], &display),
        Mark::Tone(Tone::Warning)
    );
}

#[test]
fn stars_and_hidden_accounts_are_flagged() {
    let mut state = sample();
    state.accounts[2].hidden = true;
    let display = Display {
        starred_accounts: vec![state.accounts[0].id.clone()],
        ..Display::default()
    };
    let listed = items(&state, &display);
    assert!(listed[0].starred && !listed[0].hidden);
    assert!(listed[2].hidden && !listed[2].starred);
    assert_eq!(listed.iter().filter(|item| item.starred).count(), 1);
}

#[test]
fn selection_survives_updates_and_moves_to_a_neighbour_when_the_account_leaves() {
    let state = sample();
    let listed = items(&state, &Display::default());
    let order: Vec<String> = listed.iter().map(|item| item.account_id.clone()).collect();
    let id = order[3].clone();
    assert_eq!(kept_selection(&listed, &order, Some(&id)), Some(id.clone()));
    assert_eq!(
        kept_selection(&listed, &order, None),
        Some(order[0].clone())
    );
    let fewer: Vec<SidebarItem> = listed
        .iter()
        .filter(|item| item.account_id != id)
        .cloned()
        .collect();
    assert_eq!(
        kept_selection(&fewer, &order, Some(&id)),
        Some(order[4].clone())
    );
    let last = order[5].clone();
    let without_last = &listed[..5];
    assert_eq!(
        kept_selection(without_last, &order, Some(&last)),
        Some(order[4].clone())
    );
    assert_eq!(
        kept_selection(&listed, &[], Some("gone")),
        Some(order[0].clone())
    );
    assert_eq!(kept_selection(&[], &order, Some(&id)), None);
}

#[test]
fn moves_stop_at_the_ends() {
    let order: Vec<String> = ["a", "b", "c"].map(String::from).to_vec();
    assert_eq!(
        account_moves(&order, "a"),
        Moves {
            up: false,
            down: true
        }
    );
    assert_eq!(
        account_moves(&order, "b"),
        Moves {
            up: true,
            down: true
        }
    );
    assert_eq!(
        account_moves(&order, "c"),
        Moves {
            up: true,
            down: false
        }
    );
    assert_eq!(account_moves(&order, "x"), Moves::default());
    assert_eq!(account_moves(&order[..1], "a"), Moves::default());
}

#[test]
fn sign_in_is_offered_for_signed_out_accounts_and_login_recoveries() {
    let state = sample();
    assert!(offers_sign_in(&state.accounts[3]));
    assert!(!offers_sign_in(&state.accounts[2]));
    assert!(!offers_sign_in(&state.accounts[0]));
    let mut login = state.accounts[0].clone();
    login.recovery = RecoveryField::Offered(Recovery::CliLogin {
        command: "claude auth login".into(),
    });
    assert!(offers_sign_in(&login));
    login.recovery = RecoveryField::Offered(Recovery::SignIn { account_id: None });
    assert!(offers_sign_in(&login));
}

#[test]
fn links_carry_the_current_incident_on_the_status_page() {
    let state = sample();
    let claude = provider(ProviderLinks {
        status: Some("https://status.claude.com".into()),
        dashboard: Some("https://console.anthropic.com".into()),
        usage: None,
    });
    let status = state
        .provider_status
        .iter()
        .find(|status| status.provider == "claude");
    let links = account_links(Lang::En, Some(&claude), status);
    assert_eq!(links.len(), 2);
    assert_eq!(links[0].title, "Status page");
    assert_eq!(
        links[0].subtitle,
        "Elevated errors on Claude Code · status.claude.com"
    );
    assert!(links[0].alert);
    assert_eq!(links[1].kind, LinkKind::Dashboard);
    assert_eq!(links[1].subtitle, "console.anthropic.com");
    assert!(!links[1].alert);
    let calm = account_links(Lang::En, Some(&claude), None);
    assert_eq!(calm[0].subtitle, "status.claude.com");
    assert!(account_links(Lang::En, None, status).is_empty());
}
