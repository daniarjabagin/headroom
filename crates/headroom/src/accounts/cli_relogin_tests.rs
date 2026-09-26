use std::fs;

use headroom_core::account::{AccountId, CredentialOwner};
use headroom_providers::test_support::install_script;

use super::*;

fn login_of(provider: &str) -> &'static CliLogin {
    registry::descriptor(provider)
        .and_then(ProviderDescriptor::cli_login)
        .unwrap()
}

fn words(argv: &[OsString]) -> Vec<String> {
    argv.iter()
        .map(|word| word.to_string_lossy().into_owned())
        .collect()
}

fn cli_account(provider: &str, home: &Path) -> AccountRef {
    AccountRef {
        id: AccountId(format!("{provider}:0123456789ab")),
        provider: ProviderId::parse(provider).unwrap(),
        home: home.to_path_buf(),
        owner: CredentialOwner::Cli,
    }
}

#[test]
fn the_default_home_runs_the_login_as_typed() {
    let user = Path::new("/home/ada");
    let login = terminal_login(login_of("claude"), &user.join(".claude"), Some(user)).unwrap();
    assert_eq!(
        words(&login.argv),
        [
            "env",
            "-u",
            "CLAUDE_CONFIG_DIR",
            "claude",
            "auth",
            "login",
            "--claudeai"
        ]
    );
    assert_eq!(login.display, "claude auth login --claudeai");
}

#[test]
fn a_custom_home_is_named_to_the_login() {
    let user = Path::new("/home/ada");
    let home = Path::new("/work/claude alt");
    let login = terminal_login(login_of("claude"), home, Some(user)).unwrap();
    assert_eq!(
        words(&login.argv),
        [
            "env",
            "CLAUDE_CONFIG_DIR=/work/claude alt",
            "claude",
            "auth",
            "login",
            "--claudeai"
        ]
    );
    assert_eq!(
        login.display,
        "CLAUDE_CONFIG_DIR='/work/claude alt' claude auth login --claudeai"
    );
    let codex = terminal_login(login_of("codex"), home, None).unwrap();
    assert_eq!(words(&codex.argv)[1], "CODEX_HOME=/work/claude alt");
}

#[test]
fn xdg_homes_name_their_base_and_scrub_redirecting_variables() {
    let user = Path::new("/home/ada");
    let kilo = login_of("kilo");
    let default = terminal_login(kilo, &user.join(".local/share/kilo"), Some(user)).unwrap();
    assert_eq!(
        words(&default.argv)[..7],
        [
            "env",
            "-u",
            "KILO_API_URL",
            "-u",
            "KILO_AUTH_CONTENT",
            "-u",
            "XDG_DATA_HOME"
        ]
    );
    let custom = terminal_login(kilo, Path::new("/data/alt/kilo"), Some(user)).unwrap();
    assert!(words(&custom.argv).contains(&"XDG_DATA_HOME=/data/alt".to_owned()));
    assert!(
        custom
            .display
            .starts_with("XDG_DATA_HOME=/data/alt kilo auth login")
    );
    let app = Path::new("/home/ada/.config/Devin/User/globalStorage");
    let refused = terminal_login(login_of("devin"), app, Some(user)).unwrap_err();
    assert!(refused.to_string().contains("sign in again in the app"));
}

struct Bin(tempfile::TempDir);

impl Bin {
    fn new() -> Bin {
        Bin(tempfile::tempdir().unwrap())
    }

    fn install(&self, name: &str, body: &str) {
        let path = self.0.path().join(name);
        install_script(&path, &format!("#!/bin/sh\n{body}\n")).unwrap();
    }

    fn terminals(&self) -> Terminals {
        Terminals {
            search_path: Some(self.0.path().as_os_str().to_owned()),
            preferred: None,
        }
    }

    fn path(&self, name: &str) -> PathBuf {
        self.0.path().join(name)
    }
}

#[test]
fn the_terminal_runs_the_login_in_the_accounts_home() {
    let bin = Bin::new();
    let home = tempfile::tempdir().unwrap();
    let fake_bin = bin.0.path().display().to_string();
    bin.install(
        "xdg-terminal-exec",
        &format!("PATH=\"{fake_bin}:$PATH\" exec \"$@\" </dev/null >/dev/null 2>&1"),
    );
    bin.install(
        "claude",
        "printf '%s' \"$*\" > \"$CLAUDE_CONFIG_DIR/.credentials.json\"",
    );
    let account = cli_account("claude", home.path());
    let opened = open_login(&account, login_of("claude"), &bin.terminals(), None).unwrap();
    assert_eq!(opened.terminal, bin.path("xdg-terminal-exec"));
    assert_eq!(opened.account_id, "claude:0123456789ab");
    let written = home.path().join(".credentials.json");
    for _ in 0..100 {
        if written.is_file() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    assert_eq!(
        fs::read_to_string(written).unwrap(),
        "auth login --claudeai"
    );
    assert!(opened.message().contains("claude:0123456789ab"));
}

#[test]
fn without_a_terminal_the_command_is_printed_instead() {
    let bin = Bin::new();
    let home = Path::new("/work/claude-alt");
    let account = cli_account("claude", home);
    let error = open_login(&account, login_of("claude"), &bin.terminals(), None).unwrap_err();
    assert_eq!(
        error.to_string(),
        "No terminal found to sign claude:0123456789ab in again; run \
         `CLAUDE_CONFIG_DIR=/work/claude-alt claude auth login --claudeai` in a terminal yourself"
    );
}

#[test]
fn accounts_without_a_cli_login_or_with_a_key_point_elsewhere() {
    let cursor = cli_account("cursor", Path::new("/home/ada/.cursor"));
    assert_eq!(
        sign_in_in_terminal(&cursor, false).unwrap_err().to_string(),
        "This account belongs to Cursor outside Headroom — sign in there again"
    );
    let codex = cli_account("codex", Path::new("/home/ada/.codex"));
    assert_eq!(
        sign_in_in_terminal(&codex, true).unwrap_err().to_string(),
        "This account belongs to the Codex CLI — run `codex login` instead"
    );
}
