use headroom_daemon::state::payload::{AccountError, UpdateView};
use headroom_daemon::update::InstallKind;

use super::*;
use crate::render::fixtures::full_state;

const FULL_PLAIN: &str = "\
Codex · Work  Pro
  Weekly limit shared with Codex Cloud
  Session  ━━━━━━━━┃───────────   45% left  resets in 2h 0m    ~8% spare
  Credits  $12.50

Claude · ada@claude.example  Pro  signed out
  sign-in expired, open the CLI to sign in again
  Session  ━━┃─────────────────    8% left  resets in 30m      limit in 23m

Spend  estimated from local logs
  Today      $0.01  6.2K tokens
  Yesterday  $0.00   615 tokens  partial
  30 days    $0.01  6.8K tokens  partial

1 hidden account · headroom accounts show <ID>
";

#[test]
fn renders_accounts_windows_and_spend() {
    assert_eq!(render_status(&full_state(), Palette::plain()), FULL_PLAIN);
}

#[test]
fn a_newer_release_is_mentioned_last() {
    let mut state = full_state();
    state.update = Some(UpdateView {
        version: "0.5.0".into(),
        url: "https://github.com/daniarjabagin/headroom/releases/tag/v0.5.0".into(),
        published_at: "2026-10-01T09:20:02Z".parse().unwrap(),
        install: InstallKind::SelfInstalled,
        command: "headroom update".into(),
    });
    let out = render_status(&state, Palette::plain());
    assert!(out.ends_with(
        "1 hidden account · headroom accounts show <ID>\n\nHeadroom 0.5.0 is available · headroom update\n"
    ));
}

#[test]
fn colors_follow_the_daemon_tone() {
    let out = render_status(&full_state(), Palette::colored());
    assert!(out.contains("\x1b[38;2;255;214;10m 45% left\x1b[0m"));
    assert!(out.contains("\x1b[38;2;255;69;58m  8% left\x1b[0m"));
    assert!(out.contains("\x1b[1mCodex · Work\x1b[0m"));
}

#[test]
fn empty_state_explains_what_to_do() {
    let mut state = full_state();
    state.accounts.clear();
    state.usage.clear();
    state.headline = None;
    assert_eq!(
        render_status(&state, Palette::plain()),
        format!("{NO_ACCOUNTS}\n")
    );
}

#[test]
fn accounts_without_subscription_say_so() {
    let mut state = full_state();
    state.accounts.truncate(1);
    state.usage.clear();
    let account = &mut state.accounts[0];
    account.windows.clear();
    account.balances.clear();
    account.notices.clear();
    account.plan = None;
    account.status = AccountStatus::NoSubscription;
    account.error = Some(AccountError {
        kind: "no_subscription".into(),
        message: "No active ChatGPT subscription (Free plan).".into(),
    });
    assert_eq!(
        render_status(&state, Palette::plain()),
        "Codex · Work  no active subscription\n  No active ChatGPT subscription (Free plan).\n"
    );
}

#[test]
fn accounts_without_data_say_so() {
    let mut state = full_state();
    state.accounts.truncate(1);
    state.usage.clear();
    let account = &mut state.accounts[0];
    account.windows.clear();
    account.balances.clear();
    account.notices.clear();
    account.label = None;
    account.email = None;
    account.plan = None;
    account.status = AccountStatus::Refreshing;
    assert_eq!(
        render_status(&state, Palette::plain()),
        "Codex  refreshing…\n  no data yet\n"
    );
}

#[test]
fn balances_in_other_currencies_keep_their_currency() {
    let mut state = full_state();
    state.accounts.truncate(1);
    state.usage.clear();
    let account = &mut state.accounts[0];
    account.windows.clear();
    account.notices.clear();
    account.balances = vec![
        money_balance("CNY", 12_500_000),
        money_balance("GBP", -1_250_000),
    ];
    assert_eq!(
        render_status(&state, Palette::plain()),
        "Codex · Work  Pro\n  Balance  ¥12.50\n  Balance  -1.25 GBP\n"
    );
}

fn money_balance(currency: &str, micros: i64) -> BalanceView {
    BalanceView {
        id: format!("total_{}", currency.to_ascii_lowercase()),
        label: "Balance".into(),
        amount: BalanceAmountView::Money {
            currency: currency.into(),
            micros,
        },
    }
}

const INJECTED: &str = "\x1b]0;owned\x07\x1b[2J\u{9b}31m";

fn injected_state() -> StatePayload {
    let mut state = full_state();
    state.update = Some(UpdateView {
        version: format!("0.5.0{INJECTED}"),
        url: "https://example.invalid".into(),
        published_at: "2026-10-01T09:20:02Z".parse().unwrap(),
        install: InstallKind::SelfInstalled,
        command: format!("headroom update{INJECTED}"),
    });
    let account = &mut state.accounts[0];
    account.label = Some(format!("Work{INJECTED}"));
    account.plan = Some(format!("Pro{INJECTED}"));
    account.notices[0].text = format!("note{INJECTED}");
    account.windows[0].label = format!("Session{INJECTED}");
    account.balances[0].label = format!("Credits{INJECTED}");
    account.error = Some(AccountError {
        kind: "network".into(),
        message: format!("failed{INJECTED}"),
    });
    state
}

#[test]
fn provider_and_label_text_cannot_inject_terminal_control_sequences() {
    let plain = render_status(&injected_state(), Palette::plain());
    assert!(
        !plain.chars().any(|c| c.is_control() && c != '\n'),
        "{plain:?}"
    );
    assert!(plain.contains("Codex · Work]0;owned[2J31m  Pro]0;owned[2J31m"));
    assert!(plain.contains("  Session]0;owned[2J31m  "));
    let colored = render_status(&injected_state(), Palette::colored());
    assert!(!colored.contains("\x1b]"));
    assert!(!colored.contains("\x1b[2J"));
    assert!(!colored.contains('\u{9b}'));
    assert!(!colored.contains('\x07'));
}
