use std::path::{Path, PathBuf};

use async_trait::async_trait;
use jiff::SignedDuration;
use serde::{Deserialize, Serialize};

use crate::account::{AccountRef, ProviderKind};
use crate::cursor::LogCursors;
use crate::event::UsageEvent;
use crate::quota::LimitsSnapshot;

#[async_trait]
pub trait Provider: Send + Sync {
    fn kind(&self) -> ProviderKind;
    async fn discover(&self) -> Result<Vec<AccountRef>, ProviderError>;
    /// Every directory whose local logs hold usage, with or without a signed-in account.
    async fn usage_homes(&self) -> Result<Vec<PathBuf>, ProviderError>;
    async fn fetch_limits(&self, account: &AccountRef) -> Result<LimitsSnapshot, ProviderError>;
    fn read_usage(
        &self,
        home: &Path,
        cursors: &mut LogCursors,
    ) -> Result<Vec<UsageEvent>, ProviderError>;
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind", content = "detail")]
pub enum ProviderError {
    #[error("not signed in")]
    NotSignedIn,
    #[error("sign-in expired, open the CLI to sign in again")]
    SignInExpired,
    #[error("signed in with an API key, which has no plan limits")]
    ApiKeyOnly,
    #[error("rate limited by the provider")]
    RateLimited { retry_after: Option<SignedDuration> },
    #[error("network error: {0}")]
    Network(String),
    #[error("invalid response: {0}")]
    InvalidResponse(String),
    #[error("local data error: {0}")]
    LocalData(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn messages_are_user_safe() {
        assert_eq!(ProviderError::NotSignedIn.to_string(), "not signed in");
        assert_eq!(
            ProviderError::Network("timeout".into()).to_string(),
            "network error: timeout"
        );
    }

    #[test]
    fn errors_serialize_with_kind_tag() {
        let limited = ProviderError::RateLimited {
            retry_after: Some(SignedDuration::from_secs(300)),
        };
        let json = serde_json::to_string(&limited).unwrap();
        assert_eq!(
            serde_json::from_str::<ProviderError>(&json).unwrap(),
            limited
        );
        assert_eq!(
            serde_json::to_string(&ProviderError::LocalData("x".into())).unwrap(),
            "{\"kind\":\"local_data\",\"detail\":\"x\"}"
        );
    }
}
