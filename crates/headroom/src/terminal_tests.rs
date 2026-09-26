use std::fs;

use headroom_providers::test_support::install_script;

use super::*;

struct Bin(tempfile::TempDir);

impl Bin {
    fn new() -> Bin {
        Bin(tempfile::tempdir().unwrap())
    }

    fn install(&self, name: &str, body: &str) -> PathBuf {
        let path = self.0.path().join(name);
        install_script(&path, &format!("#!/bin/sh\n{body}\n")).unwrap();
        path
    }

    fn recorder(&self, name: &str) -> PathBuf {
        let log = self.0.path().join(format!("{name}.args"));
        self.install(
            name,
            &format!("printf '%s\\n' \"$@\" > '{}'", log.display()),
        )
    }

    fn recorded(&self, name: &str) -> Vec<String> {
        let text = fs::read_to_string(self.0.path().join(format!("{name}.args"))).unwrap();
        text.lines().map(str::to_owned).collect()
    }

    fn terminals(&self, preferred: Option<&str>) -> Terminals {
        Terminals {
            search_path: Some(self.0.path().as_os_str().to_owned()),
            preferred: preferred.map(OsString::from),
        }
    }
}

fn command() -> Vec<OsString> {
    ["env", "-u", "TOOL_HOME", "tool", "login"]
        .into_iter()
        .map(OsString::from)
        .collect()
}

fn held(lead: &[&str]) -> Vec<String> {
    let tail = [
        "sh",
        "-c",
        HOLD_SCRIPT,
        "sh",
        "env",
        "-u",
        "TOOL_HOME",
        "tool",
        "login",
    ];
    lead.iter()
        .chain(tail.iter())
        .map(|w| (*w).to_owned())
        .collect()
}

#[test]
fn xdg_terminal_exec_comes_first() {
    let bin = Bin::new();
    let xdg = bin.recorder("xdg-terminal-exec");
    bin.recorder("kgx");
    let opened = bin
        .terminals(Some("kgx"))
        .open_linux_terminal(&command())
        .unwrap();
    assert_eq!(opened, Some(Opened { terminal: xdg }));
    assert_eq!(bin.recorded("xdg-terminal-exec"), held(&[]));
}

#[test]
fn the_preferred_terminal_comes_before_the_known_ones() {
    let bin = Bin::new();
    bin.recorder("kgx");
    let foot = bin.recorder("foot");
    let opened = bin
        .terminals(Some("foot"))
        .open_linux_terminal(&command())
        .unwrap();
    assert_eq!(opened, Some(Opened { terminal: foot }));
    assert_eq!(bin.recorded("foot"), held(&[]));
}

#[test]
fn a_terminal_that_fails_at_once_hands_over_to_the_next() {
    let bin = Bin::new();
    bin.install("xdg-terminal-exec", "exit 3");
    let kgx = bin.recorder("kgx");
    let opened = bin.terminals(None).open_linux_terminal(&command()).unwrap();
    assert_eq!(opened, Some(Opened { terminal: kgx }));
    assert_eq!(bin.recorded("kgx"), held(&["--"]));
}

#[test]
fn every_terminal_failing_is_an_error() {
    let bin = Bin::new();
    bin.install("xterm", "exit 1");
    let error = bin
        .terminals(None)
        .open_linux_terminal(&command())
        .unwrap_err();
    assert!(error.to_string().starts_with("no terminal could be opened"));
    assert_eq!(bin.terminals(None).candidates().len(), 1);
    let empty = Bin::new();
    assert_eq!(
        empty
            .terminals(None)
            .open_linux_terminal(&command())
            .unwrap(),
        None
    );
}

#[test]
fn each_terminal_gets_the_command_its_way() {
    let cases = [
        ("gnome-terminal", vec!["--"]),
        ("ptyxis", vec!["--new-window", "--"]),
        ("konsole", vec!["-e"]),
        ("alacritty", vec!["-e"]),
        ("kitty", vec![]),
        ("wezterm", vec!["start", "--"]),
        ("my-term", vec!["-e"]),
    ];
    for (name, lead) in cases {
        let args = terminal_args(style_of(OsStr::new(name)), &command());
        let args: Vec<String> = args
            .iter()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
        assert_eq!(args, held(&lead), "{name}");
    }
    assert_eq!(style_of(OsStr::new("/usr/bin/foot")), Style::Trailing);
}

#[test]
fn a_preferred_terminal_may_be_a_path() {
    let bin = Bin::new();
    let custom = bin.recorder("my-term");
    let terminals = Terminals {
        search_path: None,
        preferred: Some(custom.clone().into_os_string()),
    };
    assert_eq!(terminals.candidates(), [(custom, Style::DashE)]);
}

#[test]
fn the_macos_script_quotes_every_word() {
    let command: Vec<OsString> = ["env", "CLAUDE_CONFIG_DIR=/Users/ada/my claude", "claude"]
        .into_iter()
        .map(OsString::from)
        .collect();
    assert_eq!(
        command_script(&command).unwrap(),
        "#!/bin/sh\nrm -f \"$0\"\nenv 'CLAUDE_CONFIG_DIR=/Users/ada/my claude' claude\n"
    );
    assert_eq!(shell_quote("it's"), r"'it'\''s'");
    assert_eq!(shell_quote(""), "''");
}
