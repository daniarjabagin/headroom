use super::*;
use crate::payload::parse_state;

const SAMPLE: &str = include_str!("../../../../shell/gnome/dev/sample-state.json");

fn sample() -> State {
    parse_state(SAMPLE).unwrap()
}

#[test]
fn refresh_labels_in_both_languages() {
    assert_eq!(refresh_label(Lang::En, 60), "Every minute");
    assert_eq!(refresh_label(Lang::En, 300), "Every 5 minutes");
    assert_eq!(refresh_label(Lang::En, 90), "Every 90 seconds");
    assert_eq!(refresh_label(Lang::Ru, 60), "Каждую минуту");
    assert_eq!(refresh_label(Lang::Ru, 120), "Каждые 2 минуты");
    assert_eq!(refresh_label(Lang::Ru, 3600), "Каждые 60 минут");
    assert_eq!(refresh_label(Lang::Ru, 1260), "Каждую 21 минуту");
}

#[test]
fn refresh_choices_keep_a_custom_interval() {
    let values: Vec<u32> = refresh_choices(Lang::En, 300)
        .into_iter()
        .map(|choice| choice.value)
        .collect();
    assert_eq!(values, [60, 120, 300, 600, 900, 1800, 3600]);
    let values: Vec<u32> = refresh_choices(Lang::En, 450)
        .into_iter()
        .map(|choice| choice.value)
        .collect();
    assert_eq!(values, [60, 120, 300, 450, 600, 900, 1800, 3600]);
}

#[test]
fn headline_choices_list_visible_windows() {
    let state = sample();
    let choices = headline_choices(Lang::En, Some(&state), &Headline::Auto);
    assert_eq!(choices[0].value, Headline::Auto);
    assert_eq!(choices[0].label, "Auto — most critical");
    assert_eq!(choices[1].label, "Codex: work — Session");
    assert_eq!(
        choices[1].value,
        Headline::Pinned {
            account_id: "codex:1a2b3c4d5e6f".into(),
            window: "session".into()
        }
    );
    assert!(choices.iter().any(|choice| choice.label == "Grok — Weekly"));
    assert_eq!(choices.len(), 10);
}

#[test]
fn a_missing_pin_stays_selectable() {
    let pin = Headline::Pinned {
        account_id: "gone:1".into(),
        window: "weekly".into(),
    };
    let choices = headline_choices(Lang::Ru, None, &pin);
    assert_eq!(choices.len(), 2);
    assert_eq!(choices[1].value, pin);
    assert_eq!(choices[1].label, "Закреплённый лимит (сейчас недоступен)");
}

#[test]
fn account_rows_describe_owner_and_status() {
    let state = sample();
    let accounts = &state.accounts;
    assert_eq!(account_name(&accounts[0]), "work");
    assert_eq!(account_name(&accounts[4]), "OpenRouter");
    assert_eq!(
        account_subtitle(Lang::En, &accounts[0]),
        "Codex · Pro · dev@example.com"
    );
    assert_eq!(
        account_subtitle(Lang::En, &accounts[1]),
        "Codex · Plus · me@example.org · added in Headroom · Outdated"
    );
    assert_eq!(
        account_subtitle(Lang::En, &accounts[3]),
        "Claude · Team · dev@example.com · Signed out"
    );
}

#[test]
fn removal_wording_depends_on_the_owner() {
    let state = sample();
    let cli = &state.accounts[0];
    assert_eq!(
        removal_body(Lang::En, cli),
        "Headroom will stop showing this account. The Codex CLI stays signed in; you can sign in again through Headroom."
    );
    assert!(removal_body(Lang::En, &state.accounts[1]).starts_with("Headroom deletes the sign-in"));
    assert_eq!(
        removal_subtitle(Lang::Ru, Owner::Cli),
        "Аккаунт перестанет отображаться. Вход в CLI сохранится."
    );
}

#[test]
fn moving_an_account_swaps_neighbours() {
    let ids: Vec<String> = ["a", "b", "c"].map(String::from).to_vec();
    assert_eq!(moved_order(&ids, "b", -1).unwrap(), ["b", "a", "c"]);
    assert_eq!(moved_order(&ids, "b", 1).unwrap(), ["a", "c", "b"]);
    assert_eq!(moved_order(&ids, "a", -1), None);
    assert_eq!(moved_order(&ids, "c", 1), None);
    assert_eq!(moved_order(&ids, "x", 1), None);
    assert_eq!(moved_order(&ids, "a", 0), None);
}
