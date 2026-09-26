use super::*;
use crate::payload::{State, parse_state};

const SAMPLE: &str = include_str!("../../../../shell/gnome/dev/sample-state.json");
const COMBINED: &str =
    include_str!("../../../headroom-daemon/src/state/snapshots/state_combined.json");

fn sample() -> State {
    parse_state(SAMPLE).unwrap()
}

fn member(provider: &'static str, attention: Option<Attention>) -> MoreMember<'static> {
    MoreMember {
        provider,
        name: provider,
        attention,
    }
}

#[test]
fn summarizes_collapsed_cards() {
    let members = [
        member("copilot", None),
        member("grok", None),
        member("warp", None),
        member("grok", None),
        member("kimi", None),
    ];
    let summary = more_summary(Lang::En, &members).unwrap();
    assert_eq!(summary.title, "5 more");
    assert_eq!(summary.names, "copilot, grok, warp, kimi");
    assert_eq!(summary.providers, ["copilot", "grok", "warp"]);
    assert_eq!(summary.attention, None);
    assert_eq!(more_summary(Lang::Ru, &members).unwrap().title, "Ещё 5");
    assert!(more_summary(Lang::En, &[]).is_none());
}

#[test]
fn accounts_with_notices_need_attention_first() {
    let state = sample();
    let accounts = &state.accounts;
    assert_eq!(account_attention(&accounts[0], false), None);
    assert_eq!(
        account_attention(&accounts[1], false),
        Some(Attention::Tone(Tone::Critical))
    );
    assert_eq!(
        account_attention(&accounts[2], false),
        Some(Attention::Notice)
    );
    assert_eq!(
        account_attention(&accounts[3], false),
        Some(Attention::Notice)
    );
    assert_eq!(account_attention(&accounts[5], false), None);
}

#[test]
fn hidden_windows_do_not_raise_attention() {
    let mut state = sample();
    let account = &mut state.accounts[1];
    for window in &mut account.windows {
        window.hidden = window.tone == Tone::Critical;
    }
    assert_eq!(
        account_attention(account, false),
        Some(Attention::Tone(Tone::Warning))
    );
}

#[test]
fn groups_take_member_notices_and_their_own_tones() {
    let mut state = parse_state(COMBINED).unwrap();
    let group = state.combined[0].clone();
    let accounts: Vec<&Account> = state.accounts.iter().collect();
    assert_eq!(group_attention(&group, &accounts, false), None);
    let mut warned = group.clone();
    warned.windows[1].tone = Tone::Warning;
    assert_eq!(
        group_attention(&warned, &accounts, false),
        Some(Attention::Tone(Tone::Warning))
    );
    let member = state
        .accounts
        .iter_mut()
        .find(|account| account.id == "codex:personal")
        .unwrap();
    member.status = crate::payload::Status::SignedOut;
    let accounts: Vec<&Account> = state.accounts.iter().collect();
    assert_eq!(
        group_attention(&group, &accounts, false),
        Some(Attention::Notice)
    );
}

#[test]
fn the_more_row_shows_the_strongest_mark_and_counts_cards() {
    let toned = [
        member("codex", Some(Attention::Tone(Tone::Warning))),
        member("grok", None),
        member("claude", Some(Attention::Tone(Tone::Critical))),
    ];
    let attention = more_summary(Lang::En, &toned).unwrap().attention.unwrap();
    assert_eq!(attention.mark, Attention::Tone(Tone::Critical));
    assert_eq!(attention.text, "2 need attention");
    let noticed = [
        member("claude", Some(Attention::Tone(Tone::Critical))),
        member("codex", Some(Attention::Notice)),
    ];
    let attention = more_summary(Lang::Ru, &noticed).unwrap().attention.unwrap();
    assert_eq!(attention.mark, Attention::Notice);
    assert_eq!(attention.text, "2 требуют внимания");
    let one = [member("codex", Some(Attention::Notice))];
    let attention = more_summary(Lang::En, &one).unwrap().attention.unwrap();
    assert_eq!(attention.text, "1 needs attention");
    let russian = more_summary(Lang::Ru, &one).unwrap().attention.unwrap();
    assert_eq!(russian.text, "1 требует внимания");
}
