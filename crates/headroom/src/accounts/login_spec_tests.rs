use headroom_core::descriptor::{AddAccountMethod, HomeVar};

use super::test_support::{FakeBin, homes, spec};
use super::*;

static XDG_TOOL: ProviderDescriptor = ProviderDescriptor {
    id: ProviderId::from_static("xdgtool"),
    display_name: "XDG Tool",
    add_account: &[AddAccountMethod::CliLogin(CliLogin {
        program: "xdgtool",
        args: &["auth", "login"],
        home_var: HomeVar::XdgBase {
            var: "XDG_DATA_HOME",
            subdir: "xdgtool",
        },
        credentials_file: "auth.json",
        default_dir: ".tool",
        needs_pty: false,
        scrub_env: &[],
    })],
    multi_account: true,
    local_usage: false,
    links: headroom_core::descriptor::ProviderLinks::NONE,
};

static PTY_LOGIN: CliLogin = CliLogin {
    program: "ptytool",
    args: &["login"],
    home_var: HomeVar::Direct("PTYTOOL_DIR"),
    credentials_file: "auth.json",
    default_dir: ".tool",
    needs_pty: true,
    scrub_env: &[],
};

fn xdg_spec() -> LoginSpec {
    match &XDG_TOOL.add_account[0] {
        AddAccountMethod::CliLogin(login) => LoginSpec::new(&XDG_TOOL, login),
        other => panic!("{other:?}"),
    }
}

#[test]
fn login_commands_point_the_cli_at_the_new_home() {
    let home = Path::new("/data/headroom/accounts/codex/1");
    assert_eq!(
        spec("codex").display(home),
        "CODEX_HOME=/data/headroom/accounts/codex/1 codex login"
    );
    assert_eq!(
        spec("claude").display(home),
        "CLAUDE_CONFIG_DIR=/data/headroom/accounts/codex/1 claude auth login --claudeai"
    );
}

#[test]
fn xdg_base_logins_point_the_base_at_the_home_and_wait_for_the_subdir() {
    let bin = FakeBin::new();
    let root = tempfile::tempdir().unwrap();
    let spec = xdg_spec();
    assert_eq!(
        spec.display(Path::new("/h")),
        "XDG_DATA_HOME=/h xdgtool auth login"
    );
    bin.install("xdgtool", r#"echo '{}' > "$XDG_DATA_HOME/auth.json""#);
    let misplaced = sign_in(
        root.path(),
        spec,
        &bin.launcher(),
        Console::Terminal,
        &Cancel::default(),
    )
    .unwrap();
    assert!(!spec.login.credentials_path(&misplaced).exists());
    assert!(misplaced.join("auth.json").is_file());
    discard_home(&misplaced);
    bin.install(
        "xdgtool",
        r#"PATH=/usr/bin:/bin; mkdir -p "$XDG_DATA_HOME/xdgtool" && echo '{}' > "$XDG_DATA_HOME/xdgtool/auth.json""#,
    );
    let home = sign_in(
        root.path(),
        spec,
        &bin.launcher(),
        Console::Terminal,
        &Cancel::default(),
    )
    .unwrap();
    assert_eq!(home.parent().unwrap(), root.path().join("xdgtool"));
    assert!(home.join("xdgtool/auth.json").is_file());
}

const TTY_ONLY_LOGIN: &str = r#"
PATH=/usr/bin:/bin
[ -t 0 ] && [ -t 1 ] || { echo "needs an interactive terminal" >&2; exit 7; }
echo "Open https://auth.example/device?code=ABCD in your browser"
printf 'Code: '
read code
echo "got $code"
echo '{}' > "$PTYTOOL_DIR/auth.json"
"#;

fn pty_spec() -> LoginSpec {
    LoginSpec {
        provider: &XDG_TOOL.id,
        login: &PTY_LOGIN,
    }
}

#[test]
fn logins_that_need_a_terminal_run_in_a_pseudo_terminal() {
    let bin = FakeBin::new();
    let root = tempfile::tempdir().unwrap();
    bin.install("ptytool", TTY_ONLY_LOGIN);
    let mut events = Vec::new();
    let mut record = |event: LoginEvent| {
        events.push(event);
        Ok(())
    };
    let console = Console::Streamed {
        input: Box::new(std::io::Cursor::new("secret-code\n")),
        events: &mut record,
    };
    let home = sign_in(
        root.path(),
        pty_spec(),
        &bin.launcher(),
        console,
        &Cancel::default(),
    )
    .unwrap();
    assert!(home.join("auth.json").is_file());
    assert!(events.contains(&LoginEvent::Url(
        "https://auth.example/device?code=ABCD".into()
    )));
    let lines: Vec<&str> = events
        .iter()
        .filter_map(|event| match event {
            LoginEvent::Output(line) => Some(line.as_str()),
            _ => None,
        })
        .collect();
    assert!(
        lines.iter().any(|line| line.ends_with("got secret-code")),
        "{lines:?}"
    );
    assert_eq!(
        lines
            .iter()
            .filter(|line| line.contains("secret-code"))
            .count(),
        1,
        "input must not be echoed: {lines:?}"
    );
}

#[test]
fn the_same_login_fails_without_a_terminal() {
    let bin = FakeBin::new();
    let root = tempfile::tempdir().unwrap();
    bin.install("xdgtool", TTY_ONLY_LOGIN);
    let mut ignore = |_: LoginEvent| Ok(());
    let console = Console::Streamed {
        input: Box::new(std::io::empty()),
        events: &mut ignore,
    };
    let error = sign_in(
        root.path(),
        xdg_spec(),
        &bin.launcher(),
        console,
        &Cancel::default(),
    )
    .unwrap_err();
    assert!(error.to_string().contains("exit status: 7"), "{error}");
    assert!(homes(root.path(), "xdgtool").is_empty());
}

const FAKE_CLINE_LOGIN: &str = r#"
PATH=/usr/bin:/bin
[ -t 0 ] || { echo "OAuth login requires an interactive terminal session" >&2; exit 1; }
printf '%s\n' "$@" > "$CLINE_DIR/args"
echo "[auth] Enter this code in your browser: WXYZ-1234"
echo "https://auth.example/device?user_code=WXYZ-1234"
mkdir -p "$CLINE_DIR/data/settings"
echo '{}' > "$CLINE_DIR/data/settings/providers.json"
"#;

#[test]
fn cline_signs_in_through_a_pseudo_terminal_into_its_own_home() {
    let bin = FakeBin::new();
    let root = tempfile::tempdir().unwrap();
    bin.install("cline", FAKE_CLINE_LOGIN);
    let mut events = Vec::new();
    let mut record = |event: LoginEvent| {
        events.push(event);
        Ok(())
    };
    let console = Console::Streamed {
        input: Box::new(std::io::empty()),
        events: &mut record,
    };
    let home = sign_in(
        root.path(),
        spec("cline"),
        &bin.launcher(),
        console,
        &Cancel::default(),
    )
    .unwrap();
    assert_eq!(home.parent().unwrap(), root.path().join("cline"));
    assert_eq!(
        std::fs::read_to_string(home.join("args")).unwrap(),
        "auth\ncline\n"
    );
    assert!(events.contains(&LoginEvent::Url(
        "https://auth.example/device?user_code=WXYZ-1234".into()
    )));
    assert!(events.contains(&LoginEvent::Output(
        "[auth] Enter this code in your browser: WXYZ-1234".into()
    )));
}

const FAKE_CLINE_DATA_DIR: &str = r#"
PATH=/usr/bin:/bin
data="${CLINE_DATA_DIR:-$CLINE_DIR/data}"
mkdir -p "$data/settings"
echo '{}' > "$data/settings/providers.json"
"#;

#[test]
fn inherited_variables_that_redirect_a_login_are_removed() {
    let elsewhere = tempfile::tempdir().unwrap();
    let status = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "--ignored", "--quiet"])
        .arg("accounts::login::spec_tests::cline_login_with_a_foreign_data_dir")
        .env("CLINE_DATA_DIR", elsewhere.path())
        .status()
        .unwrap();
    assert!(status.success());
    assert!(!elsewhere.path().join("settings").exists());
}

#[test]
#[ignore = "run with CLINE_DATA_DIR set by inherited_variables_that_redirect_a_login_are_removed"]
fn cline_login_with_a_foreign_data_dir() {
    let bin = FakeBin::new();
    let root = tempfile::tempdir().unwrap();
    bin.install("cline", FAKE_CLINE_DATA_DIR);
    let mut record = |_: LoginEvent| Ok(());
    let console = Console::Streamed {
        input: Box::new(std::io::empty()),
        events: &mut record,
    };
    let home = sign_in(
        root.path(),
        spec("cline"),
        &bin.launcher(),
        console,
        &Cancel::default(),
    )
    .unwrap();
    assert!(home.join("data/settings/providers.json").exists());
}

fn alive(pid: &str) -> bool {
    std::fs::read_to_string(format!("/proc/{}/stat", pid.trim())).is_ok_and(|stat| {
        stat.rsplit_once(") ")
            .is_some_and(|(_, rest)| !rest.starts_with('Z'))
    })
}

fn cancel_when_written(path: PathBuf, cancel: Cancel) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        while !std::fs::read_to_string(&path).is_ok_and(|text| text.ends_with('\n')) {
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        cancel.cancel();
    })
}

#[test]
fn cancelling_a_pseudo_terminal_login_stops_it_and_removes_the_home() {
    let bin = FakeBin::new();
    let root = tempfile::tempdir().unwrap();
    let pids = tempfile::tempdir().unwrap();
    let pid_file = pids.path().join("child");
    let script = format!("echo $$ > {}\nexec /bin/sleep 300", pid_file.display());
    bin.install("ptytool", &script);
    let cancel = Cancel::default();
    let watcher = cancel_when_written(pid_file.clone(), cancel.clone());
    let mut ignore = |_: LoginEvent| Ok(());
    let console = Console::Streamed {
        input: Box::new(std::io::empty()),
        events: &mut ignore,
    };
    let error = sign_in(root.path(), pty_spec(), &bin.launcher(), console, &cancel).unwrap_err();
    watcher.join().unwrap();
    assert_eq!(error.to_string(), "cancelled");
    assert!(homes(root.path(), "xdgtool").is_empty());
    assert!(!alive(&std::fs::read_to_string(&pid_file).unwrap()));
}
