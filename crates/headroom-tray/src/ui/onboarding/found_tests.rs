use super::*;
use crate::payload::parse_state;
use crate::preferences::registry::ProviderLinks;

const FULL: &str = include_str!("../../../../headroom-daemon/src/state/snapshots/state_full.json");

fn state() -> State {
    parse_state(FULL).unwrap()
}

fn provider(id: &str, name: &str, method: AddMethod) -> ProviderInfo {
    ProviderInfo {
        id: id.into(),
        display_name: name.into(),
        methods: vec![method],
        links: ProviderLinks::default(),
    }
}

fn cli() -> AddMethod {
    AddMethod::CliLogin { program: None }
}

fn registry() -> Vec<ProviderInfo> {
    let key = AddMethod::ApiKey {
        label: None,
        console_url: None,
        hint: None,
    };
    vec![
        provider("codex", "Codex", cli()),
        provider("claude", "Claude", cli()),
        provider("cursor", "Cursor", cli()),
        provider("zai", "Z.ai", key),
        provider("copilot", "GitHub Copilot", cli()),
        provider("gemini", "Gemini", cli()),
        provider("amp", "Amp", cli()),
    ]
}

#[test]
fn lists_accounts_then_tools_that_are_not_installed() {
    let rows = found_rows(Lang::En, &state(), &registry());
    assert_eq!(rows[0].title, "Codex · Work");
    assert_eq!(rows[0].subtitle, "Signed in · Pro · ada@example.com");
    assert_eq!(
        rows[0].found,
        Found::SignedIn {
            account_id: "codex:work".into(),
            hidden: false
        }
    );
    assert_eq!(rows[1].title, "Claude");
    assert_eq!(rows[1].subtitle, "Found, not signed in");
    assert_eq!(rows[1].found, Found::SignedOut);
    assert_eq!(
        rows[2].found,
        Found::SignedIn {
            account_id: "codex:hidden".into(),
            hidden: true
        }
    );
    let missing: Vec<&str> = rows[3..].iter().map(|row| row.title.as_str()).collect();
    assert_eq!(missing, ["Cursor", "GitHub Copilot", "Gemini"]);
    assert!(rows[3..].iter().all(|row| row.found == Found::NotInstalled));
}

#[test]
fn a_usage_home_without_an_account_counts_as_found() {
    let mut state = state();
    state
        .accounts
        .retain(|account| account.provider != "claude");
    let rows = found_rows(Lang::Ru, &state, &registry());
    let claude = rows.iter().find(|row| row.provider == "claude").unwrap();
    assert_eq!(claude.found, Found::SignedOut);
    assert_eq!(claude.subtitle, "Найден, вход не выполнен");
}

#[test]
fn opens_only_for_new_daemons_that_have_not_finished_it() {
    let state = state();
    let mut settings = Settings::default();
    assert!(should_open(Some(&state), Some(&settings), false));
    assert!(!should_open(Some(&state), Some(&settings), true));
    assert!(!should_open(None, Some(&settings), false));
    assert!(!should_open(Some(&state), None, false));
    settings.onboarding.completed = true;
    assert!(!should_open(Some(&state), Some(&settings), false));
    let mut old = state.clone();
    old.panel_items = None;
    assert!(!should_open(Some(&old), Some(&Settings::default()), false));
}
