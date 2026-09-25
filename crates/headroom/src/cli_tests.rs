use super::*;

fn refresh_args(args: &[&str]) -> Result<RefreshArgs, clap::Error> {
    let parsed = Cli::try_parse_from([&["headroom", "refresh"], args].concat())?;
    match parsed.command {
        Command::Refresh(refresh) => Ok(refresh),
        other => panic!("parsed {other:?}"),
    }
}

fn daemon_args(args: &[&str]) -> DaemonArgs {
    let parsed = Cli::try_parse_from([&["headroom", "daemon"], args].concat()).unwrap();
    match parsed.command {
        Command::Daemon(daemon) => daemon,
        other => panic!("parsed {other:?}"),
    }
}

#[test]
fn the_socket_flag_takes_an_optional_path() {
    assert_eq!(daemon_args(&[]).socket, None);
    assert_eq!(daemon_args(&["--socket"]).socket, Some(None));
    assert_eq!(
        daemon_args(&["--socket", "/tmp/h.sock"]).socket,
        Some(Some(PathBuf::from("/tmp/h.sock")))
    );
}

fn update_args(args: &[&str]) -> Result<UpdateArgs, clap::Error> {
    let parsed = Cli::try_parse_from([&["headroom", "update"], args].concat())?;
    match parsed.command {
        Command::Update(update) => Ok(update),
        other => panic!("parsed {other:?}"),
    }
}

#[test]
fn update_check_excludes_installing() {
    let install = update_args(&["--yes", "--progress", "json"]).unwrap();
    assert!(install.yes && !install.check);
    assert_eq!(install.progress, Some(ProgressFormat::Json));
    assert!(update_args(&["--check"]).unwrap().check);
    assert!(update_args(&["--check", "--yes"]).is_err());
    assert!(update_args(&["--check", "--progress", "json"]).is_err());
}

#[test]
fn update_checks_can_be_turned_off_for_the_daemon() {
    assert!(!daemon_args(&[]).no_update_check);
    assert!(daemon_args(&["--no-update-check"]).no_update_check);
}

fn spend_args(args: &[&str]) -> Result<SpendArgs, clap::Error> {
    let parsed = Cli::try_parse_from([&["headroom", "spend"], args].concat())?;
    match parsed.command {
        Command::Spend(spend) => Ok(spend),
        other => panic!("parsed {other:?}"),
    }
}

#[test]
fn spend_defaults_to_models_of_the_last_week() {
    let defaults = spend_args(&[]).unwrap();
    assert_eq!(defaults.by, SpendBy::Model);
    assert_eq!(defaults.since, Since::Days(7));
    assert!(defaults.until.is_none() && defaults.provider.is_none() && !defaults.json);
    let day = spend_args(&[
        "--by",
        "day",
        "--since",
        "2026-09-01",
        "--until",
        "2026-09-02",
    ]);
    assert_eq!(day.unwrap().by, SpendBy::Day);
    assert!(spend_args(&["--by", "week"]).is_err());
}

#[test]
fn spend_since_takes_any_number_of_days_and_rejects_garbage_clearly() {
    assert_eq!(
        spend_args(&["--since", "60d"]).unwrap().since,
        Since::Days(60)
    );
    assert_eq!(
        spend_args(&["--since", "14d"]).unwrap().since,
        Since::Days(14)
    );
    for bad in ["soon", "0d", "2026-02-30"] {
        let error = spend_args(&["--since", bad]).unwrap_err();
        assert_eq!(
            error.kind(),
            clap::error::ErrorKind::ValueValidation,
            "{bad}"
        );
        assert!(error.to_string().contains("--since"), "{error}");
    }
    let until = spend_args(&["--since", "2026-09-01", "--until", "2026-09-31"]);
    assert!(until.is_err());
}

#[test]
fn refresh_now_is_a_flag_that_excludes_an_account_id() {
    let now = refresh_args(&["--now"]).unwrap();
    assert!(now.now && now.account_id.is_none());
    let due = refresh_args(&[]).unwrap();
    assert!(!due.now && due.account_id.is_none());
    let one = refresh_args(&["codex:work"]).unwrap();
    assert_eq!(one.account_id.as_deref(), Some("codex:work"));
    assert!(refresh_args(&["--now", "codex:work"]).is_err());
}

fn guard_args(args: &[&str]) -> Result<GuardArgs, clap::Error> {
    let parsed = Cli::try_parse_from([&["headroom", "guard"], args].concat())?;
    match parsed.command {
        Command::Guard(guard) => Ok(guard),
        other => panic!("parsed {other:?}"),
    }
}

fn waybar_args(args: &[&str]) -> WaybarArgs {
    let parsed = Cli::try_parse_from([&["headroom", "waybar"], args].concat()).unwrap();
    match parsed.command {
        Command::Waybar(waybar) => waybar,
        other => panic!("parsed {other:?}"),
    }
}

#[test]
fn guard_needs_a_minimum_between_0_and_100() {
    assert!(guard_args(&[]).is_err());
    assert!(guard_args(&["--min", "101"]).is_err());
    let guard = guard_args(&["--min", "20"]).unwrap();
    assert_eq!((guard.min, guard.window), (20, WindowScope::Any));
    assert!(guard.provider.is_empty() && guard.account.is_none());
    assert!(!guard.json && !guard.quiet);
    assert_eq!(guard_args(&["--min", "0"]).unwrap().min, 0);
}

#[test]
fn guard_providers_repeat_or_split_on_commas() {
    let guard = guard_args(&[
        "--min",
        "5",
        "--provider",
        "claude,codex",
        "--provider",
        "grok",
        "--window",
        "weekly",
        "--account",
        "codex:work",
        "-q",
    ])
    .unwrap();
    assert_eq!(guard.provider, ["claude", "codex", "grok"]);
    assert_eq!(guard.window, WindowScope::Weekly);
    assert_eq!(guard.account.as_deref(), Some("codex:work"));
    assert!(guard.quiet);
    assert!(guard_args(&["--min", "5", "--json", "--quiet"]).is_err());
}

#[test]
fn waybar_without_flags_keeps_the_headline() {
    assert!(waybar_args(&[]).is_headline());
    let multi = waybar_args(&["--providers", "claude,codex", "--labels", "none"]);
    assert!(!multi.is_headline());
    assert_eq!(multi.providers, ["claude", "codex"]);
    assert_eq!(multi.labels, Some(LabelStyle::None));
    assert!(!waybar_args(&["--window", "session"]).is_headline());
}

#[test]
fn window_scopes_match_window_ids() {
    assert!(WindowScope::Session.includes("session"));
    assert!(!WindowScope::Session.includes("weekly"));
    assert!(WindowScope::Weekly.includes("weekly"));
    assert!(!WindowScope::Weekly.includes("model:opus"));
    assert!(WindowScope::Any.includes("model:opus"));
}

fn accounts_action(args: &[&str]) -> Result<AccountsAction, clap::Error> {
    let parsed = Cli::try_parse_from([&["headroom", "accounts"], args].concat())?;
    match parsed.command {
        Command::Accounts(AccountsArgs {
            action: Some(action),
        }) => Ok(action),
        other => panic!("parsed {other:?}"),
    }
}

#[test]
fn accounts_login_takes_an_id_and_the_add_flags() {
    let args = [
        "login",
        "codex:0123456789ab",
        "--progress",
        "json",
        "--api-key-stdin",
    ];
    match accounts_action(&args).unwrap() {
        AccountsAction::Login {
            id,
            progress,
            api_key_stdin,
        } => {
            assert_eq!(id, "codex:0123456789ab");
            assert_eq!(progress, Some(ProgressFormat::Json));
            assert!(api_key_stdin);
        }
        other => panic!("parsed {other:?}"),
    }
    assert!(accounts_action(&["login"]).is_err());
}
