use std::io;
use std::path::Path;
use std::process::{Output, Stdio};
use std::time::Duration;

use headroom_core::provider::ProviderError;
use headroom_core::secret::SecretString;
use tokio::process::Command;

use super::hosts::GITHUB_HOST;

const TOKEN_TIMEOUT: Duration = Duration::from_secs(5);
const MAX_TOKEN_LEN: usize = 4096;
const FOREIGN_TOKEN_VARS: [&str; 5] = [
    "GH_TOKEN",
    "GITHUB_TOKEN",
    "GH_ENTERPRISE_TOKEN",
    "GITHUB_ENTERPRISE_TOKEN",
    "GH_HOST",
];

/// Asks `gh` for the stored token of one account; the token is never logged or shown.
pub(super) async fn gh_token(
    program: &Path,
    config_dir: &Path,
    login: &str,
) -> Result<SecretString, ProviderError> {
    let mut command = token_command(program, config_dir, login);
    let output = match tokio::time::timeout(TOKEN_TIMEOUT, command.output()).await {
        Ok(result) => result.map_err(|error| spawn_error(&error))?,
        Err(_) => {
            return Err(ProviderError::LocalData(
                "the GitHub CLI did not answer in time".to_owned(),
            ));
        }
    };
    token_from(&output)
}

fn token_command(program: &Path, config_dir: &Path, login: &str) -> Command {
    let mut command = Command::new(program);
    command
        .args(["auth", "token", "--hostname", GITHUB_HOST, "--user", login])
        .env("GH_CONFIG_DIR", config_dir)
        .env("GH_PROMPT_DISABLED", "1")
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("NO_COLOR", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true);
    for var in FOREIGN_TOKEN_VARS {
        command.env_remove(var);
    }
    command
}

fn spawn_error(error: &io::Error) -> ProviderError {
    match error.kind() {
        io::ErrorKind::NotFound => {
            ProviderError::LocalData("the GitHub CLI (gh) is not installed".to_owned())
        }
        kind => ProviderError::LocalData(format!("cannot run the GitHub CLI: {kind}")),
    }
}

fn token_from(output: &Output) -> Result<SecretString, ProviderError> {
    if !output.status.success() {
        return Err(ProviderError::SignInExpired);
    }
    let token = std::str::from_utf8(&output.stdout)
        .map(str::trim)
        .ok()
        .filter(|token| is_token(token))
        .ok_or_else(|| {
            ProviderError::LocalData("the GitHub CLI returned no usable token".to_owned())
        })?;
    Ok(SecretString::new(token.to_owned()))
}

fn is_token(text: &str) -> bool {
    !text.is_empty() && text.len() <= MAX_TOKEN_LEN && text.bytes().all(|b| b.is_ascii_graphic())
}

#[cfg(test)]
#[path = "token_tests.rs"]
mod tests;
