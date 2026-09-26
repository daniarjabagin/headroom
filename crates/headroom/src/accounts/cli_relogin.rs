use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow, bail};
use headroom_core::account::{AccountRef, ProviderId};
use headroom_core::descriptor::{CliLogin, ProviderDescriptor};
use headroom_providers::registry;

use crate::terminal::{Terminals, shell_quote};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminalLogin {
    pub argv: Vec<OsString>,
    pub display: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum HomeSetting {
    Default,
    Custom(OsString),
}

pub fn terminal_login(
    login: &CliLogin,
    home: &Path,
    user_home: Option<&Path>,
) -> Result<TerminalLogin> {
    let setting = home_setting(login, home, user_home)?;
    let var = login.home_var.var();
    let mut argv: Vec<OsString> = vec!["env".into()];
    for name in login.scrub_env {
        argv.extend(["-u".into(), OsString::from(name)]);
    }
    let prefix = match &setting {
        HomeSetting::Default => {
            argv.extend(["-u".into(), var.into()]);
            String::new()
        }
        HomeSetting::Custom(value) => {
            let mut assignment = OsString::from(format!("{var}="));
            assignment.push(value);
            argv.push(assignment);
            format!("{var}={} ", shell_quote(&value.to_string_lossy()))
        }
    };
    argv.push(login.program.into());
    argv.extend(login.args.iter().map(OsString::from));
    Ok(TerminalLogin {
        argv,
        display: format!("{prefix}{}", login.command_line()),
    })
}

fn home_setting(login: &CliLogin, home: &Path, user_home: Option<&Path>) -> Result<HomeSetting> {
    let default = user_home.map(|user| user.join(login.default_dir));
    if default.is_some_and(|default| same_dir(&default, home)) {
        return Ok(HomeSetting::Default);
    }
    match login.home_var.value_for(home) {
        Some(value) => Ok(HomeSetting::Custom(value.into_os_string())),
        None => bail!(
            "{} is not a directory `{}` can sign in to; sign in again in the app that uses it",
            home.display(),
            login.program
        ),
    }
}

fn same_dir(left: &Path, right: &Path) -> bool {
    match (fs::canonicalize(left), fs::canonicalize(right)) {
        (Ok(left), Ok(right)) => left == right,
        _ => left == right,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminalOpened {
    pub account_id: String,
    pub terminal: PathBuf,
    pub command: String,
}

impl TerminalOpened {
    pub fn message(&self) -> String {
        format!(
            "Opened {} running `{}`; Headroom picks up the new sign-in of {} by itself",
            self.terminal.display(),
            self.command,
            self.account_id
        )
    }
}

pub fn sign_in_in_terminal(account: &AccountRef, api_key_stdin: bool) -> Result<TerminalOpened> {
    let login = registry::descriptor(account.provider.as_str())
        .and_then(ProviderDescriptor::cli_login)
        .ok_or_else(|| anyhow!(cli_owned(&account.provider)))?;
    if api_key_stdin {
        bail!(cli_owned(&account.provider));
    }
    open_login(
        account,
        login,
        &Terminals::from_env(),
        dirs::home_dir().as_deref(),
    )
}

pub fn open_login(
    account: &AccountRef,
    login: &CliLogin,
    terminals: &Terminals,
    user_home: Option<&Path>,
) -> Result<TerminalOpened> {
    let command = terminal_login(login, &account.home, user_home)?;
    let yourself = format!("run `{}` in a terminal yourself", command.display);
    match terminals.open(&command.argv) {
        Ok(Some(opened)) => Ok(TerminalOpened {
            account_id: account.id.0.clone(),
            terminal: opened.terminal,
            command: command.display,
        }),
        Ok(None) => bail!(
            "No terminal found to sign {} in again; {yourself}",
            account.id
        ),
        Err(error) => bail!("{error:#}; {yourself}"),
    }
}

pub fn cli_owned(provider: &ProviderId) -> String {
    let Some(descriptor) = registry::descriptor(provider.as_str()) else {
        return format!("This account belongs to the {provider} CLI — sign in there again");
    };
    let name = descriptor.display_name;
    match descriptor.cli_login() {
        Some(login) => format!(
            "This account belongs to the {name} CLI — run `{}` instead",
            login.command_line()
        ),
        None => format!("This account belongs to {name} outside Headroom — sign in there again"),
    }
}

#[cfg(test)]
#[path = "cli_relogin_tests.rs"]
mod tests;
