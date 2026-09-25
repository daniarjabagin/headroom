use super::*;
use crate::payload::parse_state;

const FULL: &str = include_str!("../../../../headroom-daemon/src/state/snapshots/state_full.json");

fn state() -> State {
    parse_state(FULL).unwrap()
}

fn limit(account: &str, window: &str) -> PanelLimit {
    PanelLimit {
        account_id: account.into(),
        window: window.into(),
    }
}

#[test]
fn limit_items_keep_chosen_limits_first_and_skip_hidden_windows() {
    let state = state();
    let mut display = state.display.clone();
    display.panel_limits = vec![limit("claude:main", "session"), limit("gone:1", "weekly")];
    let items = limit_items(Lang::En, &state, &display);
    let titles: Vec<(&str, &str)> = items
        .iter()
        .map(|item| (item.title.as_str(), item.subtitle.as_str()))
        .collect();
    assert_eq!(
        titles,
        [
            ("Claude — Session", "ada@claude.example · 8% left"),
            ("Pinned limit (not available now)", "gone:1 · weekly"),
            ("Codex — Session", "Work · 45% left"),
        ]
    );
    assert_eq!(items[0].provider, "claude");
}

#[test]
fn summaries_in_both_languages() {
    assert_eq!(
        limits_summary(Lang::En, 2),
        "2 of 3 chosen · shown in this order"
    );
    assert_eq!(
        limits_summary(Lang::Ru, 1),
        "Выбрано 1 из 3 · в этом порядке"
    );
    assert_eq!(
        limits_summary(Lang::En, 0),
        "None chosen · the two most critical are shown"
    );
    assert_eq!(star_label(Lang::En, true), "Always open");
    assert_eq!(star_label(Lang::Ru, false), "По запросу");
    assert_eq!(
        almost_out_subtitle(Lang::En, 20),
        "A limit drops under 20% left"
    );
}

#[test]
fn stars_list_visible_accounts() {
    let items = star_items(&state());
    let rows: Vec<(&str, &str)> = items
        .iter()
        .map(|item| (item.title.as_str(), item.subtitle.as_str()))
        .collect();
    assert_eq!(
        rows,
        [
            ("Work", "Codex · Pro · ada@example.com"),
            ("ada@claude.example", "Claude · Pro"),
        ]
    );
}

#[test]
fn provider_thresholds_summary_counts_overrides() {
    let state = state();
    let providers = account_providers(&state);
    let ids: Vec<&str> = providers.iter().map(|item| item.id.as_str()).collect();
    assert_eq!(ids, ["codex", "claude"]);
    let mut notifications = Notifications::default();
    assert_eq!(
        thresholds_summary(Lang::En, &notifications, &providers),
        "Every provider uses the default"
    );
    notifications
        .provider_thresholds
        .insert("claude".into(), 20);
    notifications
        .provider_thresholds
        .insert("unknown".into(), 0);
    assert_eq!(
        thresholds_summary(Lang::En, &notifications, &providers),
        "1 provider differs from the default"
    );
    notifications.provider_thresholds.insert("codex".into(), 0);
    assert_eq!(
        thresholds_summary(Lang::Ru, &notifications, &providers),
        "2 провайдера отличаются от общего"
    );
}
