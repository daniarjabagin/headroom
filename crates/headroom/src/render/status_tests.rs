use super::*;
use crate::render::fixtures::full_state;

const FULL_PLAIN: &str = "\
Codex · Work  Pro
  Weekly limit shared with Codex Cloud
  Session  ━━━━━━━━┃───────────   45% left  resets in 2h 0m    ~8% spare
  Weekly   ━━━━━━━━━━━━━━──────   70% left  resets in 3d 0h
  Credits  $12.50

Claude Code · ada@claude.example  Pro  signed out
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
fn colors_follow_the_daemon_tone() {
    let out = render_status(&full_state(), Palette::colored());
    assert!(out.contains("\x1b[38;2;255;214;10m 45% left\x1b[0m"));
    assert!(out.contains("\x1b[38;2;0;145;255m 70% left\x1b[0m"));
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
