use clap::Parser;
use headroom_daemon::state::payload::{AccountStatus, WindowView};

use super::*;
use crate::cli::{Cli, Command};
use crate::render::fixtures::full_state;

fn live_state() -> StatePayload {
    let mut state = full_state();
    state.accounts[1].status = AccountStatus::Fresh;
    state.accounts[1].error = None;
    state
}

fn plain() -> Palettes {
    Palettes {
        stdout: Palette::plain(),
        stderr: Palette::plain(),
    }
}

fn args(flags: &[&str]) -> GuardArgs {
    let parsed = Cli::try_parse_from([&["headroom", "guard"], flags].concat()).unwrap();
    match parsed.command {
        Command::Guard(guard) => guard,
        other => panic!("parsed {other:?}"),
    }
}

fn weekly(remaining: f64, resets_at: &str) -> WindowView {
    let state = live_state();
    WindowView {
        id: "weekly".into(),
        label: "Weekly".into(),
        remaining_percent: remaining,
        used_percent: 100.0 - remaining,
        resets_at: Some(resets_at.parse().unwrap()),
        hidden: false,
        ..state.accounts[1].windows[0].clone()
    }
}

fn weekly_state() -> StatePayload {
    let mut state = live_state();
    state.accounts[0].windows[1].hidden = false;
    state.accounts[1]
        .windows
        .push(weekly(12.0, "2026-09-24T14:00:00Z"));
    state
}

fn output(state: &StatePayload, flags: &[&str]) -> (Printed, u8) {
    let args = args(flags);
    let outcome = evaluate(state, &args);
    let printed = printed(&outcome, &args, state.generated_at, plain()).unwrap();
    (printed, outcome.exit_status())
}

fn stdout(text: &str) -> Printed {
    Printed::Stdout(text.to_owned())
}

fn checked_ids(state: &StatePayload, flags: &[&str]) -> Vec<(String, String)> {
    match evaluate(state, &args(flags)) {
        Outcome::Checked(report) => report
            .checked
            .into_iter()
            .map(|limit| (limit.account_id, limit.window))
            .collect(),
        Outcome::NoData(reason) => panic!("no data: {reason:?}"),
    }
}

#[test]
fn verdicts_over_fixture_states() {
    let full = live_state();
    let weekly = weekly_state();
    let cases: [(&StatePayload, &[&str], Printed, u8); 7] = [
        (
            &full,
            &["--min", "5"],
            stdout("✓ Limits ok · lowest Claude session 8% left (min 5%)\n"),
            0,
        ),
        (
            &full,
            &["--min", "20"],
            stdout("✗ Claude session: 8% left < 20% · resets in 30m\n"),
            1,
        ),
        (
            &weekly,
            &["--min", "20", "--window", "weekly"],
            stdout("✗ Claude weekly: 12% left < 20% · resets in 1d 4h\n"),
            1,
        ),
        (
            &weekly,
            &["--min", "20", "--window", "weekly", "--provider", "codex"],
            stdout("✓ Weekly limits ok · lowest Codex 70% left (min 20%)\n"),
            0,
        ),
        (
            &weekly,
            &["--min", "50", "--provider", "codex"],
            stdout("✗ Codex session: 45% left < 50% · resets in 2h 0m\n"),
            1,
        ),
        (
            &weekly,
            &["--min", "10", "--account", "claude:main"],
            stdout("✗ Claude session: 8% left < 10% · resets in 30m\n"),
            1,
        ),
        (&weekly, &["--min", "99", "--quiet"], Printed::Nothing, 1),
    ];
    for (state, flags, expected, status) in cases {
        assert_eq!(output(state, flags), (expected, status), "{flags:?}");
    }
}

#[test]
fn every_failing_window_gets_a_line() {
    let (printed, status) = output(&weekly_state(), &["--min", "50"]);
    let expected = "\
✗ Codex session: 45% left < 50% · resets in 2h 0m
✗ Claude session: 8% left < 50% · resets in 30m
✗ Claude weekly: 12% left < 50% · resets in 1d 4h
";
    assert_eq!((printed, status), (stdout(expected), 1));
}

#[test]
fn weekly_skips_accounts_and_windows_without_a_visible_weekly_limit() {
    assert_eq!(
        checked_ids(&weekly_state(), &["--min", "1", "--window", "weekly"]),
        [
            ("codex:work".to_owned(), "weekly".to_owned()),
            ("claude:main".to_owned(), "weekly".to_owned()),
        ]
    );
    let hidden_weekly = live_state();
    let outcome = evaluate(&hidden_weekly, &args(&["--min", "1", "--window", "weekly"]));
    assert_eq!(outcome, Outcome::NoData(NoData::NoMatchingLimits));
    assert_eq!(outcome.exit_status(), 2);
}

#[test]
fn hidden_accounts_are_never_checked() {
    let mut state = live_state();
    state.accounts[2]
        .windows
        .push(weekly(1.0, "2026-09-24T14:00:00Z"));
    let (printed, status) = output(&state, &["--min", "5"]);
    assert_eq!(status, 0, "{printed:?}");
    let only_hidden = evaluate(&state, &args(&["--min", "5", "--account", "codex:hidden"]));
    assert_eq!(only_hidden, Outcome::NoData(NoData::NoMatchingLimits));
}

#[test]
fn balances_are_not_limits() {
    let state = live_state();
    assert!(!state.accounts[0].balances.is_empty());
    assert_eq!(
        checked_ids(&state, &["--min", "1"]),
        [
            ("codex:work".to_owned(), "session".to_owned()),
            ("claude:main".to_owned(), "session".to_owned()),
        ]
    );
}

#[test]
fn unknown_providers_have_no_data() {
    let outcome = evaluate(&live_state(), &args(&["--min", "1", "--provider", "grok"]));
    assert_eq!(outcome, Outcome::NoData(NoData::NoMatchingLimits));
}

#[test]
fn accounts_of_one_provider_are_told_apart() {
    let mut state = live_state();
    let mut second = state.accounts[0].clone();
    second.id = "codex:home".into();
    second.label = Some("Home".into());
    second.windows[0].remaining_percent = 3.0;
    state.accounts.push(second);
    let (printed, _) = output(&state, &["--min", "5", "--provider", "codex"]);
    assert_eq!(
        printed,
        stdout("✗ Codex · Home session: 3% left < 5% · resets in 2h 0m\n")
    );
}

#[test]
fn json_lists_the_verdict_and_every_checked_limit() {
    let (printed, status) = output(&weekly_state(), &["--min", "20", "--json"]);
    assert_eq!(status, 1);
    let Printed::Stdout(text) = printed else {
        panic!("{printed:?}");
    };
    let json: serde_json::Value = serde_json::from_str(&text).unwrap();
    let keys: Vec<&str> = json
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(keys, ["checked", "failing", "min_percent", "ok", "window"]);
    assert_eq!(json["ok"], false);
    assert_eq!(json["min_percent"], 20);
    assert_eq!(json["window"], "any");
    assert_eq!(json["checked"].as_array().unwrap().len(), 4);
    assert_eq!(
        json["failing"],
        serde_json::json!([
            {
                "account_id": "claude:main",
                "provider": "claude",
                "window": "session",
                "remaining_percent": 8.0,
                "resets_at": "2026-09-23T10:30:00Z",
                "tone": "critical"
            },
            {
                "account_id": "claude:main",
                "provider": "claude",
                "window": "weekly",
                "remaining_percent": 12.0,
                "resets_at": "2026-09-24T14:00:00Z",
                "tone": "critical"
            }
        ])
    );
}

#[test]
fn json_keeps_the_mockup_field_order() {
    let (printed, _) = output(&live_state(), &["--min", "1", "--json"]);
    let Printed::Stdout(text) = printed else {
        panic!("{printed:?}");
    };
    let order = [
        "\"ok\"",
        "\"min_percent\"",
        "\"window\"",
        "\"checked\"",
        "\"failing\"",
    ];
    let positions: Vec<usize> = order.iter().map(|key| text.find(key).unwrap()).collect();
    assert!(positions.is_sorted(), "{text}");
}

#[test]
fn no_data_goes_to_stderr_unless_quiet() {
    let outcome = Outcome::NoData(NoData::DaemonNotRunning);
    let now = live_state().generated_at;
    let loud = printed(&outcome, &args(&["--min", "20"]), now, plain()).unwrap();
    let Printed::Stderr(text) = loud else {
        panic!("{loud:?}");
    };
    assert!(text.starts_with("headroom: no limit data · the daemon is not running"));
    let quiet = printed(&outcome, &args(&["--min", "20", "-q"]), now, plain()).unwrap();
    assert_eq!(quiet, Printed::Nothing);
}

#[test]
fn exit_status_maps_the_outcome() {
    let pass = evaluate(&live_state(), &args(&["--min", "0"]));
    let below = evaluate(&live_state(), &args(&["--min", "100"]));
    assert_eq!(pass.exit_status(), 0);
    assert_eq!(below.exit_status(), 1);
    for reason in [
        NoData::DaemonNotRunning,
        NoData::Unreadable("broken".into()),
        NoData::NoMatchingLimits,
        NoData::NoFreshLimits(vec!["Codex: data is outdated".into()]),
    ] {
        assert_eq!(Outcome::NoData(reason).exit_status(), 2);
    }
}

fn codex_with(status: AccountStatus) -> StatePayload {
    let mut state = live_state();
    state.accounts[0].status = status;
    state
}

#[test]
fn only_fresh_or_refreshing_accounts_are_checked() {
    for status in [AccountStatus::Fresh, AccountStatus::Refreshing] {
        assert_eq!(
            checked_ids(&codex_with(status), &["--min", "1", "--provider", "codex"]),
            [("codex:work".to_owned(), "session".to_owned())],
            "{status:?}"
        );
    }
    let excluded = [
        (AccountStatus::Stale, "Codex: data is outdated"),
        (AccountStatus::Error, "Codex: couldn't refresh"),
        (AccountStatus::SignedOut, "Codex: signed out"),
        (
            AccountStatus::NoSubscription,
            "Codex: no active subscription",
        ),
    ];
    for (status, reason) in excluded {
        let outcome = evaluate(
            &codex_with(status),
            &args(&["--min", "1", "--provider", "codex"]),
        );
        assert_eq!(
            outcome,
            Outcome::NoData(NoData::NoFreshLimits(vec![reason.to_owned()])),
            "{status:?}"
        );
        assert_eq!(outcome.exit_status(), 2);
    }
}

#[test]
fn a_signed_out_account_does_not_pass_the_guard() {
    let state = full_state();
    assert_eq!(
        checked_ids(&state, &["--min", "1"]),
        [("codex:work".to_owned(), "session".to_owned())]
    );
    let (printed, status) = output(&state, &["--min", "50", "--provider", "claude"]);
    assert_eq!(status, 2);
    assert_eq!(
        printed,
        Printed::Stderr(
            "headroom: no limit data · no fresh limit data · Claude: sign-in expired, open the CLI \
             to sign in again\n"
                .to_owned()
        )
    );
}

#[test]
fn every_excluded_account_is_named() {
    let mut state = codex_with(AccountStatus::Stale);
    state.accounts[1].status = AccountStatus::Error;
    let outcome = evaluate(&state, &args(&["--min", "1"]));
    assert_eq!(
        outcome,
        Outcome::NoData(NoData::NoFreshLimits(vec![
            "Codex: data is outdated".to_owned(),
            "Claude: couldn't refresh".to_owned(),
        ]))
    );
}

#[tokio::test]
async fn a_missing_daemon_exits_with_2() {
    let dir = tempfile::tempdir().unwrap();
    let globals = Globals {
        #[cfg(target_os = "linux")]
        bus: headroom_daemon::BusTarget::Session,
        db: Some(dir.path().join("state.db")),
        socket: Some(dir.path().join("absent.sock")),
    };
    assert_eq!(current_state(&globals).await, Err(NoData::DaemonNotRunning));
    let code = run(&globals, &args(&["--min", "20", "--quiet"]))
        .await
        .unwrap();
    assert_eq!(code, ExitCode::from(2));
}
