use std::fs;
use std::io::Write as _;
use std::process::Output;

use headroom_providers::test_support::install_script;

use super::*;

struct Bin(tempfile::TempDir);

impl Bin {
    fn new() -> Bin {
        Bin(tempfile::tempdir().unwrap())
    }

    fn dir(&self) -> &Path {
        self.0.path()
    }

    fn install(&self, name: &str, body: &str) -> PathBuf {
        let path = self.dir().join(name);
        install_script(&path, &format!("#!/bin/sh\n{body}\n")).unwrap();
        path
    }

    fn recorder(&self, name: &str) -> PathBuf {
        let log = self.dir().join(format!("{name}.args"));
        self.install(
            name,
            &format!("printf '%s\\0' \"$@\" > '{}'", log.display()),
        )
    }

    fn recorded(&self, name: &str) -> Vec<String> {
        let text = fs::read_to_string(self.dir().join(format!("{name}.args"))).unwrap();
        text.split_terminator('\0').map(str::to_owned).collect()
    }

    fn terminals(&self, preferred: Option<&str>) -> TerminalChain {
        TerminalChain {
            search_path: Some(self.dir().as_os_str().to_owned()),
            preferred: preferred.map(OsString::from),
        }
    }
}

fn words(list: &[&str]) -> Vec<OsString> {
    list.iter().map(OsString::from).collect()
}

fn command() -> TerminalCommand {
    TerminalCommand {
        program: "tool".into(),
        argv: words(&["env", "-u", "TOOL_HOME", "tool", "login"]),
        display: "tool login".into(),
    }
}

fn held(lead: &[&str]) -> Vec<String> {
    let tail = [
        "sh",
        "-c",
        LAUNCH_SCRIPT,
        "sh",
        HOLD_SCRIPT,
        "tool",
        "tool login",
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
    let opened = bin.terminals(Some("kgx")).open(&command()).unwrap();
    assert_eq!(opened, Some(Opened { terminal: xdg }));
    assert_eq!(bin.recorded("xdg-terminal-exec"), held(&[]));
}

#[test]
fn the_preferred_terminal_comes_before_the_known_ones() {
    let bin = Bin::new();
    bin.recorder("kgx");
    let foot = bin.recorder("foot");
    let opened = bin.terminals(Some("foot")).open(&command()).unwrap();
    assert_eq!(opened, Some(Opened { terminal: foot }));
    assert_eq!(bin.recorded("foot"), held(&[]));
}

#[test]
fn a_terminal_that_fails_at_once_hands_over_to_the_next() {
    let bin = Bin::new();
    bin.install("xdg-terminal-exec", "exit 3");
    let kgx = bin.recorder("kgx");
    let opened = bin.terminals(None).open(&command()).unwrap();
    assert_eq!(opened, Some(Opened { terminal: kgx }));
    assert_eq!(bin.recorded("kgx"), held(&["--"]));
}

#[test]
fn every_terminal_failing_is_an_error() {
    let bin = Bin::new();
    bin.install("xterm", "exit 1");
    let error = bin.terminals(None).open(&command()).unwrap_err();
    assert!(error.to_string().starts_with("no terminal could be opened"));
    assert_eq!(bin.terminals(None).candidates().len(), 1);
    let empty = Bin::new();
    assert_eq!(empty.terminals(None).open(&command()).unwrap(), None);
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
    let terminals = TerminalChain {
        search_path: None,
        preferred: Some(custom.clone().into_os_string()),
    };
    assert_eq!(terminals.candidates(), [(custom, Style::DashE)]);
}

#[test]
fn only_macos_opens_the_terminal_app() {
    let mac = matches!(Terminals::from_env(), Terminals::MacTerminal);
    assert_eq!(mac, cfg!(target_os = "macos"));
}

#[test]
fn the_macos_script_quotes_every_word() {
    let command = TerminalCommand {
        program: "claude".into(),
        argv: words(&["env", "CLAUDE_CONFIG_DIR=/Users/ada/my claude", "claude"]),
        display: "claude".into(),
    };
    let hold = shell_quote(HOLD_SCRIPT);
    assert_eq!(
        command_script(&command).unwrap(),
        format!(
            "#!/bin/sh\nrm -f \"$0\"\nset -- {hold} claude claude env \
             'CLAUDE_CONFIG_DIR=/Users/ada/my claude' claude\n{LAUNCH_SCRIPT}\n"
        )
    );
    assert_eq!(shell_quote("it's"), r"'it'\''s'");
    assert_eq!(shell_quote(""), "''");
}

const TRICKY: [&str; 5] = ["it's", "$HOME", "`id`", "a\"b", "; exit 5"];

fn tricky_command() -> TerminalCommand {
    let mut argv = words(&["env", "TOOL_HOME=/work/my $tool", "tool"]);
    argv.extend(TRICKY.iter().map(OsString::from));
    TerminalCommand {
        program: "tool".into(),
        argv,
        display: "TOOL_HOME='/work/my $tool' tool login".into(),
    }
}

fn run_launch(shell: &Path, path: &OsStr, home: &Path, command: &TerminalCommand) -> Output {
    let mut child = Command::new("/bin/sh")
        .arg("-c")
        .arg(LAUNCH_SCRIPT)
        .arg("sh")
        .args(command.launch_args())
        .env_clear()
        .env("PATH", path)
        .env("HOME", home)
        .env("SHELL", shell)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(b"\n").unwrap();
    child.wait_with_output().unwrap()
}

fn search_path(dirs: &[&Path]) -> OsString {
    let system = [Path::new("/usr/bin"), Path::new("/bin")];
    std::env::join_paths(dirs.iter().copied().chain(system)).unwrap()
}

fn tool_recorder(dir: &Bin) {
    let log = dir.dir().join("tool.args");
    dir.install(
        "tool",
        &format!(
            "printf '%s\\0' \"$TOOL_HOME\" \"$@\" > '{}'\nexit 7",
            log.display()
        ),
    );
}

fn expected_tool_args() -> Vec<String> {
    ["/work/my $tool"]
        .into_iter()
        .chain(TRICKY)
        .map(str::to_owned)
        .collect()
}

#[test]
fn a_posix_login_shell_runs_the_login_with_its_rc_files() {
    let bin = Bin::new();
    let home = tempfile::tempdir().unwrap();
    let flags = bin.dir().join("flags");
    let zsh = bin.install(
        "zsh",
        &format!(
            "printf '%s\\n' \"$1\" \"$2\" \"$3\" > '{}'\nshift 2\nexec /bin/sh \"$@\"",
            flags.display()
        ),
    );
    tool_recorder(&bin);
    let output = run_launch(
        &zsh,
        &search_path(&[bin.dir()]),
        home.path(),
        &tricky_command(),
    );
    assert_eq!(output.status.code(), Some(7));
    assert_eq!(fs::read_to_string(flags).unwrap(), "-l\n-i\n-c\n");
    assert_eq!(bin.recorded("tool"), expected_tool_args());
    assert!(String::from_utf8_lossy(&output.stdout).contains("Press Enter to close"));
}

#[test]
fn another_login_shell_lends_its_path_to_the_login() {
    let bin = Bin::new();
    let extra = Bin::new();
    let home = tempfile::tempdir().unwrap();
    let fish = bin.install(
        "fish",
        &format!(
            "[ \"$1 $2 $3\" = '-l -i -c' ] || exit 9\necho 'Welcome to fish'\n\
             PATH='{}':\"$PATH\" exec /bin/sh -c \"$4\"",
            extra.dir().display()
        ),
    );
    tool_recorder(&extra);
    let output = run_launch(
        &fish,
        &search_path(&[bin.dir()]),
        home.path(),
        &tricky_command(),
    );
    assert_eq!(output.status.code(), Some(7));
    assert_eq!(extra.recorded("tool"), expected_tool_args());
}

#[test]
fn a_login_nobody_can_find_says_so_in_the_window() {
    let bin = Bin::new();
    let home = tempfile::tempdir().unwrap();
    let broken = bin.install("nu", "exit 1");
    let output = run_launch(
        &broken,
        &search_path(&[bin.dir()]),
        home.path(),
        &tricky_command(),
    );
    assert_eq!(output.status.code(), Some(127));
    let shown = String::from_utf8_lossy(&output.stdout);
    assert!(
        shown.starts_with(
            "tool not found in PATH — install it or run `TOOL_HOME='/work/my $tool' tool login` \
             yourself\n"
        ),
        "{shown}"
    );
}
