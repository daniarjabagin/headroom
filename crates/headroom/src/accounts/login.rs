use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};
use headroom_core::account::ProviderKind;

use super::home::create_home;

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

pub fn sign_in(root: &Path, provider: ProviderKind, launcher: &Launcher) -> Result<PathBuf> {
    let home = create_home(root, provider)?;
    let spec = login_spec(provider);
    match run_login(&spec, &home, launcher) {
        Ok(()) => Ok(home),
        Err(error) => {
            if let Err(cleanup) = fs::remove_dir_all(&home) {
                tracing::warn!(home = %home.display(), %cleanup, "could not remove the new home");
            }
            Err(error)
        }
    }
}

fn run_login(spec: &LoginSpec, home: &Path, launcher: &Launcher) -> Result<()> {
    let status = launcher
        .command(spec, home)
        .status()
        .with_context(|| format!("could not start `{}`; is it installed?", spec.program))?;
    let command = format!("{} {}", spec.program, spec.args.join(" "));
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
