use std::ffi::{OsStr, OsString};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};

const LAUNCH_GRACE: Duration = Duration::from_millis(1500);
const LAUNCH_POLL: Duration = Duration::from_millis(50);
const HOLD_SCRIPT: &str = r#"program=$1; display=$2; shift 2
if command -v "$program" >/dev/null 2>&1; then
  "$@"; code=$?
else
  printf '%s not found in PATH — install it or run `%s` yourself\n' "$program" "$display"
  code=127
fi
printf '\n%s' 'Press Enter to close this window. '
read -r answer
exit "$code""#;
const LAUNCH_SCRIPT: &str = r#"hold=$1; shift
shell=${SHELL:-/bin/sh}
[ -x "$shell" ] || shell=/bin/sh
case ${shell##*/} in
  sh|bash|zsh|dash|ksh|mksh|yash) exec "$shell" -l -i -c "$hold" sh "$@" ;;
esac
found=$("$shell" -l -i -c 'printf "\nheadroom-path=%s\n" "$PATH"' </dev/null 2>/dev/null |
  sed -n 's/^headroom-path=//p' | tail -n 1)
if [ -n "$found" ]; then PATH=$found; export PATH; fi
exec /bin/sh -c "$hold" sh "$@""#;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Style {
    Trailing,
    DashE,
    DoubleDash,
    NewWindow,
    WeztermStart,
}

const KNOWN: [(&str, Style); 10] = [
    ("xdg-terminal-exec", Style::Trailing),
    ("kgx", Style::DoubleDash),
    ("ptyxis", Style::NewWindow),
    ("gnome-terminal", Style::DoubleDash),
    ("konsole", Style::DashE),
    ("foot", Style::Trailing),
    ("kitty", Style::Trailing),
    ("alacritty", Style::DashE),
    ("wezterm", Style::WeztermStart),
    ("xterm", Style::DashE),
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminalCommand {
    pub program: OsString,
    pub argv: Vec<OsString>,
    pub display: String,
}

impl TerminalCommand {
    fn launch_args(&self) -> impl Iterator<Item = OsString> + '_ {
        [
            OsString::from(HOLD_SCRIPT),
            self.program.clone(),
            OsString::from(&self.display),
        ]
        .into_iter()
        .chain(self.argv.iter().cloned())
    }
}

#[derive(Debug, Clone, Default)]
pub struct TerminalChain {
    pub search_path: Option<OsString>,
    pub preferred: Option<OsString>,
}

#[derive(Debug, Clone)]
pub enum Terminals {
    MacTerminal,
    Chain(TerminalChain),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Opened {
    pub terminal: PathBuf,
}

impl Terminals {
    pub fn from_env() -> Terminals {
        if cfg!(target_os = "macos") {
            return Terminals::MacTerminal;
        }
        Terminals::Chain(TerminalChain::from_env())
    }

    pub fn open(&self, command: &TerminalCommand) -> Result<Option<Opened>> {
        match self {
            Terminals::MacTerminal => open_macos_terminal(command).map(Some),
            Terminals::Chain(chain) => chain.open(command),
        }
    }
}

impl TerminalChain {
    fn from_env() -> TerminalChain {
        TerminalChain {
            search_path: std::env::var_os("PATH"),
            preferred: std::env::var_os("TERMINAL").filter(|value| !value.is_empty()),
        }
    }

    fn open(&self, command: &TerminalCommand) -> Result<Option<Opened>> {
        let mut failures = Vec::new();
        for (terminal, style) in self.candidates() {
            match launch(&terminal, style, command) {
                Ok(()) => return Ok(Some(Opened { terminal })),
                Err(error) => failures.push(format!("{}: {error:#}", terminal.display())),
            }
        }
        if failures.is_empty() {
            return Ok(None);
        }
        bail!("no terminal could be opened ({})", failures.join("; "))
    }

    fn candidates(&self) -> Vec<(PathBuf, Style)> {
        let preferred = self
            .preferred
            .iter()
            .map(|name| (name.clone(), style_of(name)));
        let known = KNOWN
            .iter()
            .map(|(name, style)| (OsString::from(name), *style));
        let mut found: Vec<(PathBuf, Style)> = Vec::new();
        let ordered = known.clone().take(1).chain(preferred).chain(known.skip(1));
        for (name, style) in ordered {
            let Some(path) = self.find(&name) else {
                continue;
            };
            if !found.iter().any(|(seen, _)| *seen == path) {
                found.push((path, style));
            }
        }
        found
    }

    fn find(&self, name: &OsStr) -> Option<PathBuf> {
        let name = Path::new(name);
        if name.components().count() > 1 {
            return is_executable(name).then(|| name.to_path_buf());
        }
        let search = self.search_path.as_deref()?;
        std::env::split_paths(search)
            .map(|dir| dir.join(name))
            .find(|candidate| is_executable(candidate))
    }
}

fn style_of(name: &OsStr) -> Style {
    let base = Path::new(name).file_name().unwrap_or(name);
    KNOWN
        .iter()
        .find(|(known, _)| OsStr::new(known) == base)
        .map_or(Style::DashE, |(_, style)| *style)
}

fn is_executable(path: &Path) -> bool {
    path.metadata()
        .is_ok_and(|meta| meta.is_file() && meta.permissions().mode() & 0o111 != 0)
}

fn terminal_args(style: Style, command: &TerminalCommand) -> Vec<OsString> {
    let held = ["sh", "-c", LAUNCH_SCRIPT, "sh"]
        .into_iter()
        .map(OsString::from)
        .chain(command.launch_args());
    let lead: &[&str] = match style {
        Style::Trailing => &[],
        Style::DashE => &["-e"],
        Style::DoubleDash => &["--"],
        Style::NewWindow => &["--new-window", "--"],
        Style::WeztermStart => &["start", "--"],
    };
    lead.iter().map(OsString::from).chain(held).collect()
}

fn launch(terminal: &Path, style: Style, command: &TerminalCommand) -> Result<()> {
    let mut child = Command::new(terminal)
        .args(terminal_args(style, command))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .process_group(0)
        .spawn()
        .context("could not start it")?;
    let deadline = Instant::now() + LAUNCH_GRACE;
    while Instant::now() < deadline {
        match child.try_wait().context("could not wait for it")? {
            Some(status) if status.success() => return Ok(()),
            Some(status) => bail!("it exited at once ({status})"),
            None => thread::sleep(LAUNCH_POLL),
        }
    }
    Ok(())
}

fn open_macos_terminal(command: &TerminalCommand) -> Result<Opened> {
    let script = command_script(command)?;
    let file = tempfile::Builder::new()
        .prefix("headroom-sign-in-")
        .suffix(".command")
        .tempfile()
        .context("could not create the sign-in script")?;
    std::fs::write(file.path(), script).context("could not write the sign-in script")?;
    let (_, path) = file.keep().context("could not keep the sign-in script")?;
    let opened = run_in_macos_terminal(&path);
    if opened.is_err() {
        let _ = std::fs::remove_file(&path);
    }
    opened
}

fn run_in_macos_terminal(script: &Path) -> Result<Opened> {
    std::fs::set_permissions(script, std::fs::Permissions::from_mode(0o700))
        .context("could not make the sign-in script runnable")?;
    let terminal = PathBuf::from("/usr/bin/open");
    let status = Command::new(&terminal)
        .args([OsStr::new("-a"), OsStr::new("Terminal"), script.as_os_str()])
        .status()
        .context("could not run open")?;
    if !status.success() {
        bail!("open -a Terminal failed ({status})");
    }
    Ok(Opened { terminal })
}

fn command_script(command: &TerminalCommand) -> Result<String> {
    let words = command
        .launch_args()
        .map(|word| word.to_str().map(shell_quote))
        .collect::<Option<Vec<_>>>()
        .context("the sign-in command is not valid UTF-8")?;
    Ok(format!(
        "#!/bin/sh\nrm -f \"$0\"\nset -- {}\n{LAUNCH_SCRIPT}\n",
        words.join(" ")
    ))
}

pub fn shell_quote(word: &str) -> String {
    let plain = !word.is_empty()
        && word
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "_@%+=:,./-".contains(c));
    if plain {
        word.to_owned()
    } else {
        format!("'{}'", word.replace('\'', r"'\''"))
    }
}

#[cfg(test)]
#[path = "terminal_tests.rs"]
mod tests;
