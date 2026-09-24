#[cfg(test)]
pub(crate) mod fake;

use std::io;
use std::path::PathBuf;
use std::process::{Output, Stdio};
use std::time::Duration;

use headroom_core::secret::SecretString;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

pub const SYSTEM_PROGRAM: &str = "/usr/bin/security";
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(5);

const ITEM_NOT_FOUND: i32 = 44;
const DENIED_CODES: [i32; 3] = [36, 51, 128];
const MAX_COMMAND_LINE: usize = 4032;

/// macOS `security` tool, the only way Headroom touches the Keychain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Security {
    program: PathBuf,
    timeout: Duration,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenericPassword<'a> {
    pub service: &'a str,
    pub account: Option<&'a str>,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum KeychainError {
    #[error("cannot run {}: {kind}", program.display())]
    Spawn {
        program: PathBuf,
        kind: io::ErrorKind,
    },
    #[error("the Keychain did not answer within {} s", .0.as_secs())]
    TimedOut(Duration),
    #[error("the Keychain is locked or access was denied (security exit code {0})")]
    Denied(i32),
    #[error("security failed ({})", exit_text(*.0))]
    Failed(Option<i32>),
    #[error("the Keychain item is not valid UTF-8")]
    NotUtf8,
    #[error("the Keychain {0} contains characters that cannot be passed to security")]
    UnsafeValue(&'static str),
    #[error("the secret is too long to pass to security on stdin")]
    TooLong,
    #[error("the Keychain did not keep the new item")]
    NotStored,
}

impl KeychainError {
    #[must_use]
    pub fn is_unavailable(&self) -> bool {
        matches!(
            self,
            KeychainError::Spawn { .. } | KeychainError::TimedOut(_)
        )
    }
}

impl Security {
    #[must_use]
    pub fn system() -> Security {
        Security::new(PathBuf::from(SYSTEM_PROGRAM), DEFAULT_TIMEOUT)
    }

    #[must_use]
    pub fn new(program: PathBuf, timeout: Duration) -> Security {
        Security { program, timeout }
    }

    pub async fn find(
        &self,
        item: GenericPassword<'_>,
    ) -> Result<Option<SecretString>, KeychainError> {
        let mut args = vec!["find-generic-password", "-s", item.service];
        if let Some(account) = item.account {
            args.extend(["-a", account]);
        }
        args.push("-w");
        let output = self.run(&args, None).await?;
        match exit_code(&output)? {
            Found::Yes => password(output.stdout).map(Some),
            Found::No => Ok(None),
        }
    }

    /// Creates or replaces the item; the secret goes over stdin, never on the command line.
    pub async fn store(
        &self,
        item: GenericPassword<'_>,
        label: &str,
        secret: &SecretString,
    ) -> Result<(), KeychainError> {
        let line = add_command(item, label, secret)?;
        let output = self.run(&["-i"], Some(line.into_bytes())).await?;
        if let Found::No = exit_code(&output)? {
            return Err(KeychainError::Failed(Some(ITEM_NOT_FOUND)));
        }
        match self.find(item).await? {
            Some(stored) if stored == *secret => Ok(()),
            _ => Err(KeychainError::NotStored),
        }
    }

    pub async fn delete(&self, item: GenericPassword<'_>) -> Result<bool, KeychainError> {
        let mut args = vec!["delete-generic-password", "-s", item.service];
        if let Some(account) = item.account {
            args.extend(["-a", account]);
        }
        let output = self.run(&args, None).await?;
        Ok(matches!(exit_code(&output)?, Found::Yes))
    }

    async fn run(&self, args: &[&str], input: Option<Vec<u8>>) -> Result<Output, KeychainError> {
        let running = self.spawn_and_wait(args, input);
        tokio::time::timeout(self.timeout, running)
            .await
            .map_err(|_| KeychainError::TimedOut(self.timeout))?
    }

    async fn spawn_and_wait(
        &self,
        args: &[&str],
        input: Option<Vec<u8>>,
    ) -> Result<Output, KeychainError> {
        let spawn_error = |error: io::Error| KeychainError::Spawn {
            program: self.program.clone(),
            kind: error.kind(),
        };
        let mut child = Command::new(&self.program)
            .args(args)
            .stdin(if input.is_some() {
                Stdio::piped()
            } else {
                Stdio::null()
            })
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .map_err(spawn_error)?;
        if let (Some(bytes), Some(mut stdin)) = (input, child.stdin.take()) {
            stdin.write_all(&bytes).await.map_err(spawn_error)?;
        }
        child.wait_with_output().await.map_err(spawn_error)
    }
}

enum Found {
    Yes,
    No,
}

fn exit_code(output: &Output) -> Result<Found, KeychainError> {
    match output.status.code() {
        Some(0) => Ok(Found::Yes),
        Some(ITEM_NOT_FOUND) => Ok(Found::No),
        Some(code) if DENIED_CODES.contains(&code) => Err(KeychainError::Denied(code)),
        code => Err(KeychainError::Failed(code)),
    }
}

fn password(mut stdout: Vec<u8>) -> Result<SecretString, KeychainError> {
    if stdout.last() == Some(&b'\n') {
        stdout.pop();
    }
    String::from_utf8(stdout)
        .map(SecretString::new)
        .map_err(|_| KeychainError::NotUtf8)
}

fn add_command(
    item: GenericPassword<'_>,
    label: &str,
    secret: &SecretString,
) -> Result<String, KeychainError> {
    let mut parts = vec![
        "add-generic-password -U".to_owned(),
        format!("-s {}", quoted(item.service, "service")?),
        format!("-l {}", quoted(label, "label")?),
    ];
    if let Some(account) = item.account {
        parts.push(format!("-a {}", quoted(account, "account")?));
    }
    parts.push(format!("-X \"{}\"", hex::encode(secret.expose())));
    let line = parts.join(" ") + "\n";
    if line.len() > MAX_COMMAND_LINE {
        return Err(KeychainError::TooLong);
    }
    Ok(line)
}

fn quoted(value: &str, what: &'static str) -> Result<String, KeychainError> {
    let safe = !value.is_empty()
        && value
            .chars()
            .all(|c| !c.is_control() && c != '"' && c != '\\');
    if safe {
        Ok(format!("\"{value}\""))
    } else {
        Err(KeychainError::UnsafeValue(what))
    }
}

fn exit_text(code: Option<i32>) -> String {
    code.map_or_else(
        || "killed by a signal".to_owned(),
        |code| format!("exit code {code}"),
    )
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
