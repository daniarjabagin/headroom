use std::fs;
use std::os::unix::fs::PermissionsExt;

use headroom_core::account::{AccountId, AccountRef, CredentialOwner};

use super::test_support::{FakeBin, FixedAccounts, homes, spec};
use super::*;

#[test]
fn sign_in_runs_the_cli_and_cleans_up_failures() {
    let bin = FakeBin::new();
    let root = tempfile::tempdir().unwrap();
    let launcher = bin.launcher();

    bin.install(
        "codex",
        r#"printf '%s\n' "$@" > "$CODEX_HOME/args"; echo '{}' > "$CODEX_HOME/auth.json""#,
    );
    let home = sign_in(
        root.path(),
        spec("codex"),
        &launcher,
        Console::Terminal,
        &Cancel::default(),
    )
    .unwrap();
    assert_eq!(home.parent().unwrap(), root.path().join("codex"));
    assert_eq!(fs::read_to_string(home.join("args")).unwrap(), "login\n");
    assert_eq!(
        fs::metadata(&home).unwrap().permissions().mode() & 0o777,
        0o700
    );

    bin.install(
        "claude",
        r#"printf '%s\n' "$@" > "$CLAUDE_CONFIG_DIR/args"; echo '{}' > "$CLAUDE_CONFIG_DIR/.credentials.json""#,
    );
    let home = sign_in(
        root.path(),
        spec("claude"),
        &launcher,
        Console::Terminal,
        &Cancel::default(),
    )
    .unwrap();
    assert_eq!(home.parent().unwrap(), root.path().join("claude"));
    assert_eq!(
        fs::read_to_string(home.join("args")).unwrap(),
        "auth\nlogin\n--claudeai\n"
    );

    bin.install(
        "claude",
        r#"echo partial > "$CLAUDE_CONFIG_DIR/log"; exit 3"#,
    );
    let error = sign_in(
        root.path(),
        spec("claude"),
        &launcher,
        Console::Terminal,
        &Cancel::default(),
    )
    .unwrap_err();
    assert!(
        error.to_string().contains("did not finish successfully"),
        "{error}"
    );
    assert_eq!(homes(root.path(), "claude").len(), 1);

    bin.install("codex", "exit 0");
    let unconfirmed = sign_in(
        root.path(),
        spec("codex"),
        &launcher,
        Console::Terminal,
        &Cancel::default(),
    )
    .unwrap();
    assert!(!unconfirmed.join("auth.json").exists());
    discard_home(&unconfirmed);
    assert_eq!(homes(root.path(), "codex").len(), 1);

    let empty = FakeBin::new();
    let error = sign_in(
        root.path(),
        spec("codex"),
        &empty.launcher(),
        Console::Terminal,
        &Cancel::default(),
    )
    .unwrap_err();
    assert!(error.to_string().contains("is it installed"), "{error}");
    assert_eq!(homes(root.path(), "codex").len(), 1);
}

const FAKE_CODEX_LOGIN: &str = r#"
printf '\033[1mStarting local login server on http://localhost:1455.\033[0m\n'
echo "If your browser did not open, navigate to this URL to authenticate:"
echo "https://auth.openai.com/oauth/authorize?client_id=x&state=1"
echo "https://auth.openai.com/oauth/authorize?client_id=x&state=1" >&2
read code
echo "got $code"
echo '{}' > "$CODEX_HOME/auth.json"
"#;

fn streamed_sign_in(
    root: &Path,
    provider: LoginSpec,
    launcher: &Launcher,
    input: &str,
) -> (Result<PathBuf>, Vec<LoginEvent>) {
    let mut events = Vec::new();
    let mut record = |event: LoginEvent| {
        events.push(event);
        Ok(())
    };
    let console = Console::Streamed {
        input: Box::new(std::io::Cursor::new(input.to_owned())),
        events: &mut record,
    };
    let result = sign_in(root, provider, launcher, console, &Cancel::default());
    (result, events)
}

fn outputs(events: &[LoginEvent]) -> Vec<&str> {
    events
        .iter()
        .filter_map(|event| match event {
            LoginEvent::Output(line) => Some(line.as_str()),
            _ => None,
        })
        .collect()
}

#[test]
fn streamed_sign_in_reports_output_urls_and_forwards_input() {
    let bin = FakeBin::new();
    let root = tempfile::tempdir().unwrap();
    bin.install("codex", FAKE_CODEX_LOGIN);
    let (result, events) =
        streamed_sign_in(root.path(), spec("codex"), &bin.launcher(), "abc123\n");
    let home = result.unwrap();
    assert_eq!(events[0], LoginEvent::Started(home.clone()));
    let urls: Vec<&LoginEvent> = events
        .iter()
        .filter(|event| matches!(event, LoginEvent::Url(_)))
        .collect();
    assert_eq!(
        urls,
        [&LoginEvent::Url(
            "https://auth.openai.com/oauth/authorize?client_id=x&state=1".into()
        )]
    );
    let lines = outputs(&events);
    assert!(lines.contains(&"Starting local login server on http://localhost:1455."));
    assert!(lines.contains(&"got abc123"), "{lines:?}");
    let repeated = lines
        .iter()
        .filter(|line| line.starts_with("https://auth.openai.com"))
        .count();
    assert_eq!(repeated, 2);
    assert!(home.join("auth.json").is_file());
}

#[test]
fn streamed_failures_keep_the_output_and_clean_up() {
    let bin = FakeBin::new();
    let root = tempfile::tempdir().unwrap();
    bin.install("claude", r#"echo "Login failed: denied" >&2; exit 3"#);
    let (result, events) = streamed_sign_in(root.path(), spec("claude"), &bin.launcher(), "");
    let error = result.unwrap_err();
    assert!(
        error.to_string().contains("did not finish successfully"),
        "{error}"
    );
    assert_eq!(outputs(&events), ["Login failed: denied"]);
    assert!(homes(root.path(), "claude").is_empty());

    let empty = FakeBin::new();
    let (result, events) = streamed_sign_in(root.path(), spec("codex"), &empty.launcher(), "");
    let error = result.unwrap_err();
    assert!(
        format!("{error:#}").contains("is it installed"),
        "{error:#}"
    );
    assert_eq!(events.len(), 1);
    assert!(homes(root.path(), "codex").is_empty());
}

#[test]
fn a_failing_sink_stops_the_login() {
    let bin = FakeBin::new();
    let root = tempfile::tempdir().unwrap();
    bin.install("codex", "echo first\nexec sleep 30");
    let mut failing = |event: LoginEvent| match event {
        LoginEvent::Output(_) => Err(anyhow::anyhow!("stdout closed")),
        LoginEvent::Started(_) | LoginEvent::Url(_) => Ok(()),
    };
    let console = Console::Streamed {
        input: Box::new(std::io::empty()),
        events: &mut failing,
    };
    let started = std::time::Instant::now();
    let error = sign_in(
        root.path(),
        spec("codex"),
        &bin.launcher(),
        console,
        &Cancel::default(),
    )
    .unwrap_err();
    assert!(error.to_string().contains("stdout closed"), "{error}");
    assert!(started.elapsed() < std::time::Duration::from_secs(10));
    assert!(homes(root.path(), "codex").is_empty());
}

fn running(pid: &str) -> bool {
    fs::read_to_string(format!("/proc/{}/stat", pid.trim()))
        .ok()
        .and_then(|stat| {
            stat.rsplit_once(") ")
                .map(|(_, rest)| !rest.starts_with('Z'))
        })
        .unwrap_or(false)
}

fn wait_for_file(path: &Path) -> String {
    for _ in 0..200 {
        if let Ok(text) = fs::read_to_string(path)
            && text.ends_with('\n')
        {
            return text;
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    panic!("{} never appeared", path.display());
}

fn eventually_stopped(pid: &str) -> bool {
    (0..200).any(|_| {
        std::thread::sleep(std::time::Duration::from_millis(25));
        !running(pid)
    })
}

fn install_hanging_login(bin: &FakeBin, pids: &Path) {
    let pids = pids.display();
    bin.install(
        "codex",
        &format!("PATH=/usr/bin:/bin\nsleep 300 &\necho $! > {pids}/grandchild\necho $$ > {pids}/child\nwait"),
    );
}

fn cancelled_sign_in(
    console: Console<'_>,
    bin: &FakeBin,
    root: &Path,
    pids: &Path,
) -> Result<PathBuf> {
    let cancel = Cancel::default();
    let remote = cancel.clone();
    let child_file = pids.join("child");
    let watcher = std::thread::spawn(move || {
        wait_for_file(&child_file);
        remote.cancel();
    });
    let result = sign_in(root, spec("codex"), &bin.launcher(), console, &cancel);
    watcher.join().unwrap();
    result
}

#[test]
fn cancelling_a_streamed_login_kills_its_process_group_and_home() {
    let bin = FakeBin::new();
    let root = tempfile::tempdir().unwrap();
    let pids = tempfile::tempdir().unwrap();
    install_hanging_login(&bin, pids.path());
    let mut ignore = |_: LoginEvent| Ok(());
    let console = Console::Streamed {
        input: Box::new(std::io::empty()),
        events: &mut ignore,
    };
    let error = cancelled_sign_in(console, &bin, root.path(), pids.path()).unwrap_err();
    assert_eq!(error.to_string(), "cancelled");
    assert!(homes(root.path(), "codex").is_empty());
    let child = fs::read_to_string(pids.path().join("child")).unwrap();
    let grandchild = wait_for_file(&pids.path().join("grandchild"));
    assert!(!running(&child));
    assert!(eventually_stopped(&grandchild));
}

#[test]
fn cancelling_a_terminal_login_stops_the_cli_and_removes_the_home() {
    let bin = FakeBin::new();
    let root = tempfile::tempdir().unwrap();
    let pids = tempfile::tempdir().unwrap();
    install_hanging_login(&bin, pids.path());
    let error = cancelled_sign_in(Console::Terminal, &bin, root.path(), pids.path()).unwrap_err();
    assert_eq!(error.to_string(), "cancelled");
    assert!(homes(root.path(), "codex").is_empty());
    let child = fs::read_to_string(pids.path().join("child")).unwrap();
    assert!(!running(&child));
    let grandchild = wait_for_file(&pids.path().join("grandchild"));
    let pid = rustix::process::Pid::from_raw(grandchild.trim().parse().unwrap()).unwrap();
    rustix::process::kill_process(pid, rustix::process::Signal::KILL).ok();
}

fn claude_at(home: &Path) -> AccountRef {
    AccountRef {
        id: AccountId("claude:0123456789ab".into()),
        provider: spec("claude").provider.clone(),
        home: home.to_path_buf(),
        owner: CredentialOwner::Headroom,
    }
}

#[tokio::test]
async fn sign_ins_are_confirmed_by_the_credentials_file_or_the_provider() {
    let root = tempfile::tempdir().unwrap();
    let claude = spec("claude");
    let with_file = root.path().join("file");
    fs::create_dir_all(&with_file).unwrap();
    fs::write(claude.login.credentials_path(&with_file), "{}").unwrap();
    confirm_sign_in(&FixedAccounts(Vec::new()), &claude, &with_file)
        .await
        .unwrap();
    let in_keychain = root.path().join("keychain");
    fs::create_dir_all(&in_keychain).unwrap();
    let provider = FixedAccounts(vec![claude_at(&in_keychain)]);
    confirm_sign_in(&provider, &claude, &in_keychain)
        .await
        .unwrap();
    assert!(in_keychain.is_dir());
    let missing = root.path().join("missing");
    fs::create_dir_all(&missing).unwrap();
    let error = confirm_sign_in(&provider, &claude, &missing)
        .await
        .unwrap_err();
    assert!(
        error.to_string().contains("wrote no .credentials.json"),
        "{error}"
    );
    assert!(!missing.exists());
}
