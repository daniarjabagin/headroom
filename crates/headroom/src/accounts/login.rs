use std::collections::HashSet;
use std::ffi::OsString;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

use anyhow::{Context, Result, bail};
use headroom_core::account::ProviderKind;

use super::ansi::CleanLine;
use super::home::create_home;
use super::stream::run_streamed;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LoginSpec {
    pub program: &'static str,
    pub args: &'static [&'static str],
    pub home_var: &'static str,
    pub credentials_file: &'static str,
}

pub fn login_spec(provider: ProviderKind) -> LoginSpec {
    match provider {
        ProviderKind::Codex => LoginSpec {
            program: "codex",
            args: &["login"],
            home_var: "CODEX_HOME",
            credentials_file: "auth.json",
        },
        ProviderKind::Claude => LoginSpec {
            program: "claude",
            args: &["auth", "login", "--claudeai"],
            home_var: "CLAUDE_CONFIG_DIR",
            credentials_file: ".credentials.json",
        },
    }
}

impl LoginSpec {
    pub fn display(&self, home: &Path) -> String {
        format!(
            "{}={} {} {}",
            self.home_var,
            home.display(),
            self.program,
            self.args.join(" ")
        )
    }

    fn command_line(&self) -> String {
        format!("{} {}", self.program, self.args.join(" "))
    }
}

#[derive(Debug, Clone, Default)]
pub struct Launcher {
    pub search_path: Option<OsString>,
}

impl Launcher {
    fn command(&self, spec: &LoginSpec, home: &Path) -> Command {
        let mut command = Command::new(spec.program);
        command.args(spec.args).env(spec.home_var, home);
        if let Some(path) = &self.search_path {
            command.env("PATH", path);
        }
        command
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoginEvent {
    Started(PathBuf),
    Output(String),
    Url(String),
}

pub type EventSink<'a> = dyn FnMut(LoginEvent) -> Result<()> + 'a;

pub enum Console<'a> {
    Terminal,
    Streamed {
        input: Box<dyn Read + Send>,
        events: &'a mut EventSink<'a>,
    },
}

pub fn sign_in(
    root: &Path,
    provider: ProviderKind,
    launcher: &Launcher,
    console: Console<'_>,
) -> Result<PathBuf> {
    let home = create_home(root, provider)?;
    let spec = login_spec(provider);
    match run_login(&spec, &home, launcher, console) {
        Ok(()) => Ok(home),
        Err(error) => {
            if let Err(cleanup) = fs::remove_dir_all(&home) {
                tracing::warn!(home = %home.display(), %cleanup, "could not remove the new home");
            }
            Err(error)
        }
    }
}

fn run_login(
    spec: &LoginSpec,
    home: &Path,
    launcher: &Launcher,
    console: Console<'_>,
) -> Result<()> {
    let command = launcher.command(spec, home);
    let status = match console {
        Console::Terminal => spawn_attached(command)?,
        Console::Streamed { input, events } => {
            events(LoginEvent::Started(home.to_path_buf()))?;
            spawn_streamed(command, input, events)?
        }
    };
    check_outcome(spec, home, status)
}

fn spawn_attached(mut command: Command) -> Result<ExitStatus> {
    command.status().with_context(|| not_started(&command))
}

pub fn not_started(command: &Command) -> String {
    let program = command.get_program().to_string_lossy();
    format!("could not start `{program}`; is it installed?")
}

fn spawn_streamed(
    command: Command,
    input: Box<dyn Read + Send>,
    events: &mut EventSink<'_>,
) -> Result<ExitStatus> {
    let mut seen_urls = HashSet::new();
    let mut on_line = |raw: &str| {
        let line = CleanLine::parse(raw);
        let url = line.url();
        events(LoginEvent::Output(line.text))?;
        match url {
            Some(url) if seen_urls.insert(url.clone()) => events(LoginEvent::Url(url)),
            _ => Ok(()),
        }
    };
    run_streamed(command, input, &mut on_line)
}

fn check_outcome(spec: &LoginSpec, home: &Path, status: ExitStatus) -> Result<()> {
    let command = spec.command_line();
    if !status.success() {
        bail!("`{command}` did not finish successfully ({status})");
    }
    if !home.join(spec.credentials_file).is_file() {
        bail!(
            "`{command}` finished but wrote no {}",
            spec.credentials_file
        );
    }
    Ok(())
}

#[cfg(test)]
#[path = "login_tests.rs"]
mod tests;
