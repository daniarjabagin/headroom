use std::collections::HashSet;
use std::ffi::OsString;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus};
use std::thread;
use std::time::Duration;

use anyhow::{Context, Result, bail};
use headroom_core::account::ProviderId;
use headroom_core::descriptor::{CliLogin, ProviderDescriptor};
use headroom_core::provider::Provider;
use sha2::{Digest, Sha256};

use super::ansi::CleanLine;
use super::cancel::{CANCELLED, Cancel};
use super::home::{create_home, discard_home};
use super::stream::{LineSink, run_in_pty, run_streamed};

const EXIT_POLL: Duration = Duration::from_millis(100);

#[derive(Debug, Clone, Copy)]
pub struct LoginSpec {
    pub provider: &'static ProviderId,
    pub login: &'static CliLogin,
}

impl LoginSpec {
    pub fn new(descriptor: &'static ProviderDescriptor, login: &'static CliLogin) -> LoginSpec {
        LoginSpec {
            provider: &descriptor.id,
            login,
        }
    }

    pub fn display(&self, home: &Path) -> String {
        let var = self.login.home_var.var();
        format!("{var}={} {}", home.display(), self.command_line())
    }

    fn command_line(&self) -> String {
        let mut line = self.login.program.to_owned();
        for arg in self.login.args {
            line.push(' ');
            line.push_str(arg);
        }
        line
    }
}

#[derive(Debug, Clone, Default)]
pub struct Launcher {
    pub search_path: Option<OsString>,
}

impl Launcher {
    fn command(&self, spec: &LoginSpec, home: &Path) -> Command {
        let login = spec.login;
        let mut command = Command::new(login.program);
        for name in login.scrub_env {
            command.env_remove(name);
        }
        command.args(login.args).env(login.home_var.var(), home);
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

type Runner = fn(Command, Box<dyn Read + Send>, &mut LineSink<'_>, &Cancel) -> Result<ExitStatus>;

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
    spec: LoginSpec,
    launcher: &Launcher,
    console: Console<'_>,
    cancel: &Cancel,
) -> Result<PathBuf> {
    let home = create_home(root, spec.provider)?;
    match sign_in_at(&home, spec, launcher, console, cancel) {
        Ok(()) => Ok(home),
        Err(error) => {
            discard_home(&home);
            Err(error)
        }
    }
}

pub fn sign_in_at(
    home: &Path,
    spec: LoginSpec,
    launcher: &Launcher,
    console: Console<'_>,
    cancel: &Cancel,
) -> Result<()> {
    let command = launcher.command(&spec, home);
    let status = match console {
        Console::Terminal => spawn_attached(command, cancel)?,
        Console::Streamed { input, events } => {
            events(LoginEvent::Started(home.to_path_buf()))?;
            let runner = if spec.login.needs_pty {
                run_in_pty
            } else {
                run_streamed
            };
            spawn_streamed(runner, command, input, events, cancel)?
        }
    };
    check_outcome(&spec, status)
}

fn spawn_attached(mut command: Command, cancel: &Cancel) -> Result<ExitStatus> {
    let mut child = command.spawn().with_context(|| not_started(&command))?;
    loop {
        if cancel.is_cancelled() {
            stop_attached(&mut child);
            bail!(CANCELLED);
        }
        if let Some(status) = child.try_wait().context("could not wait for the login")? {
            return Ok(status);
        }
        thread::sleep(EXIT_POLL);
    }
}

fn stop_attached(child: &mut Child) {
    if let Err(error) = child.kill() {
        tracing::debug!(%error, "login process already gone");
    }
    if let Err(error) = child.wait() {
        tracing::debug!(%error, "could not reap the login process");
    }
}

pub fn not_started(command: &Command) -> String {
    let program = command.get_program().to_string_lossy();
    format!("could not start `{program}`; is it installed?")
}

fn spawn_streamed(
    runner: Runner,
    command: Command,
    input: Box<dyn Read + Send>,
    events: &mut EventSink<'_>,
    cancel: &Cancel,
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
    runner(command, input, &mut on_line, cancel)
}

fn check_outcome(spec: &LoginSpec, status: ExitStatus) -> Result<()> {
    if !status.success() {
        bail!(
            "`{}` did not finish successfully ({status})",
            spec.command_line()
        );
    }
    Ok(())
}

pub async fn confirm_sign_in(provider: &dyn Provider, spec: &LoginSpec, home: &Path) -> Result<()> {
    if spec.login.credentials_path(home).is_file() {
        return Ok(());
    }
    let found = provider.account_at(home).await;
    if matches!(found, Ok(Some(_))) {
        return Ok(());
    }
    discard_home(home);
    let missing = format!(
        "`{}` finished but wrote no {}",
        spec.command_line(),
        spec.login.credentials_file
    );
    match found {
        Err(error) => Err(error).context(missing),
        Ok(_) => bail!(missing),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CredentialsStamp(Option<[u8; 32]>);

impl CredentialsStamp {
    pub fn read(spec: &LoginSpec, home: &Path) -> Result<CredentialsStamp> {
        let path = spec.login.credentials_path(home);
        match fs::read(&path) {
            Ok(bytes) => Ok(CredentialsStamp(Some(Sha256::digest(&bytes).into()))),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(CredentialsStamp(None)),
            Err(error) => Err(error).with_context(|| format!("could not read {}", path.display())),
        }
    }
}

pub async fn confirm_renewal(
    provider: &dyn Provider,
    spec: &LoginSpec,
    home: &Path,
    before: &CredentialsStamp,
) -> Result<()> {
    let after = CredentialsStamp::read(spec, home)?;
    let renewed = match after.0 {
        Some(_) => after != *before,
        None => matches!(provider.account_at(home).await, Ok(Some(_))),
    };
    if !renewed {
        bail!(
            "`{}` finished but did not renew {}",
            spec.command_line(),
            spec.login.credentials_file
        );
    }
    Ok(())
}

#[cfg(test)]
#[path = "login_test_support.rs"]
mod test_support;

#[cfg(test)]
#[path = "login_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "login_spec_tests.rs"]
mod spec_tests;

#[cfg(test)]
#[path = "login_renewal_tests.rs"]
mod renewal_tests;
