use std::fmt;
use std::fs::File;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use headroom_core::provider::ProviderError;
use jiff::Timestamp;
use serde::Deserialize;

use super::config::CursorConfig;
use super::jwt;
use super::state_db;

const ACCESS_TOKEN_KEY: &str = "cursorAuth/accessToken";
const MEMBERSHIP_KEY: &str = "cursorAuth/stripeMembershipType";
const EMAIL_KEY: &str = "cursorAuth/cachedEmail";
const FREE_MEMBERSHIP: &str = "free";
const MAX_AGENT_FILE_BYTES: u64 = 64 * 1024;

pub(super) struct AccessToken(String);

impl AccessToken {
    #[cfg(test)]
    pub(super) fn new(secret: String) -> AccessToken {
        AccessToken(secret)
    }

    pub(super) fn secret(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for AccessToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("AccessToken(<redacted>)")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Source {
    Ide,
    Agent,
}

#[derive(Debug)]
pub(super) struct Credentials {
    access_token: AccessToken,
    expires_at: Option<Timestamp>,
    pub(super) subject: String,
    pub(super) membership: Option<String>,
    pub(super) email: Option<String>,
    pub(super) source: Source,
}

impl Credentials {
    pub(super) fn usable_token(&self, now: Timestamp) -> Result<&AccessToken, ProviderError> {
        if self.expires_at.is_some_and(|at| at <= now) {
            return Err(ProviderError::SignInExpired);
        }
        Ok(&self.access_token)
    }

    pub(super) fn home(&self, config: &CursorConfig) -> PathBuf {
        match self.source {
            Source::Ide => config.ide_dir(),
            Source::Agent => config.agent_dir(),
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawAgentAuth {
    access_token: Option<String>,
}

pub(super) fn load_credentials(
    config: &CursorConfig,
) -> Result<Option<Credentials>, ProviderError> {
    let ide = load_ide(&config.state_db());
    let agent = load_agent(&config.agent_auth_file());
    match (ide, agent) {
        (Ok(Some(ide)), Ok(Some(agent))) if prefers_agent(&ide, &agent) => Ok(Some(agent)),
        (Ok(Some(ide)), _) => Ok(Some(ide)),
        (Ok(None), agent) => agent,
        (Err(error), Ok(Some(agent))) => {
            tracing::warn!(%error, "using the Cursor agent login instead of the app login");
            Ok(Some(agent))
        }
        (Err(error), _) => Err(error),
    }
}

fn prefers_agent(ide: &Credentials, agent: &Credentials) -> bool {
    let ide_is_free = ide
        .membership
        .as_deref()
        .is_some_and(|membership| membership.eq_ignore_ascii_case(FREE_MEMBERSHIP));
    ide_is_free && ide.subject != agent.subject
}

fn load_ide(path: &Path) -> Result<Option<Credentials>, ProviderError> {
    let mut items = state_db::read_items(path, &[ACCESS_TOKEN_KEY, MEMBERSHIP_KEY, EMAIL_KEY])?;
    let Some(token) = items.remove(ACCESS_TOKEN_KEY) else {
        return Ok(None);
    };
    let mut credentials = credentials(token, Source::Ide)?;
    credentials.membership = items.remove(MEMBERSHIP_KEY);
    credentials.email = items.remove(EMAIL_KEY);
    Ok(Some(credentials))
}

fn load_agent(path: &Path) -> Result<Option<Credentials>, ProviderError> {
    let Some(bytes) = read_bounded(path).map_err(|error| agent_error(&error))? else {
        return Ok(None);
    };
    let raw: RawAgentAuth = serde_json::from_slice(&bytes).map_err(|_| {
        ProviderError::LocalData("the Cursor agent auth.json is not valid JSON".into())
    })?;
    raw.access_token
        .map(|token| token.trim().to_owned())
        .filter(|token| !token.is_empty())
        .map(|token| credentials(token, Source::Agent))
        .transpose()
}

fn read_bounded(path: &Path) -> io::Result<Option<Vec<u8>>> {
    let file = match File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error),
    };
    let mut bytes = Vec::new();
    file.take(MAX_AGENT_FILE_BYTES + 1)
        .read_to_end(&mut bytes)?;
    if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > MAX_AGENT_FILE_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "file is too large",
        ));
    }
    Ok(Some(bytes))
}

fn agent_error(error: &io::Error) -> ProviderError {
    ProviderError::LocalData(format!("cannot read the Cursor agent auth.json: {error}"))
}

fn credentials(token: String, source: Source) -> Result<Credentials, ProviderError> {
    let claims = jwt::claims(&token);
    let subject = claims.subject.ok_or_else(|| {
        ProviderError::LocalData("the Cursor sign-in token does not name an account".into())
    })?;
    Ok(Credentials {
        access_token: AccessToken(token),
        expires_at: claims.expires_at,
        subject,
        membership: None,
        email: None,
        source,
    })
}

#[cfg(test)]
#[path = "auth_tests.rs"]
mod tests;
