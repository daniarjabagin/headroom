use std::path::{Path, PathBuf};

use async_trait::async_trait;
use jiff::SignedDuration;
use serde::{Deserialize, Serialize};

use crate::account::{AccountIdentity, AccountRef, ProviderId};
use crate::cursor::LogCursors;
use crate::descriptor::ProviderDescriptor;
use crate::event::UsageEvent;
use crate::quota::LimitsSnapshot;

#[async_trait]
pub trait Provider: Send + Sync {
    fn descriptor(&self) -> &'static ProviderDescriptor;

    fn id(&self) -> &'static ProviderId {
        &self.descriptor().id
    }

    /// Every signed-in home, most preferred first; one account may be listed at several homes.
    async fn discover(&self) -> Result<Vec<AccountRef>, ProviderError>;

    /// The account signed in at one Headroom-owned home, even when discovery lists it elsewhere.
    async fn account_at(&self, home: &Path) -> Result<Option<AccountRef>, ProviderError> {
        match self.discover().await {
            Ok(accounts) => Ok(accounts.into_iter().find(|account| account.home == home)),
            Err(ProviderError::NotSignedIn) => Ok(None),
            Err(error) => Err(error),
        }
    }

    /// Every directory whose local logs hold usage, with or without a signed-in account.
    async fn usage_homes(&self) -> Result<Vec<PathBuf>, ProviderError>;
    async fn fetch_limits(&self, account: &AccountRef) -> Result<LimitsSnapshot, ProviderError>;
    fn read_usage(
        &self,
        home: &Path,
        cursors: &mut LogCursors,
    ) -> Result<Vec<UsageEvent>, ProviderError>;

    /// Checks a pasted API key with the provider and returns whose key it is.
    async fn validate_key(&self, _key: &str) -> Result<AccountIdentity, ProviderError> {
        Err(ProviderError::Unsupported(format!(
            "{} accounts cannot be added with an API key",
            self.descriptor().display_name
        )))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind", content = "detail")]
pub enum ProviderError {
    #[error("not signed in")]
    NotSignedIn,
    #[error("sign-in expired, open the CLI to sign in again")]
    SignInExpired,
    #[error("{0}")]
    AccountChanged(String),
    #[error("signed in with an API key, which has no plan limits")]
    ApiKeyOnly,
    #[error("{detail}")]
    NoSubscription { detail: String },
    #[error("rate limited by the provider")]
    RateLimited { retry_after: Option<SignedDuration> },
    #[error("network error: {0}")]
    Network(String),
    #[error("invalid response: {0}")]
    InvalidResponse(String),
    #[error("local data error: {0}")]
    LocalData(String),
    #[error("{0}")]
    Unsupported(String),
}

impl ProviderError {
    /// True when discovery found no tool or sign-in, which is not a failure.
    #[must_use]
    pub fn is_nothing_to_discover(&self) -> bool {
        matches!(self, ProviderError::NotSignedIn)
    }

    /// True when signing in again, not retrying, is what clears the error.
    #[must_use]
    pub fn needs_sign_in(&self) -> bool {
        matches!(
            self,
            ProviderError::NotSignedIn | ProviderError::SignInExpired | ProviderError::ApiKeyOnly
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::account::{AccountId, CredentialOwner};
    use crate::descriptor::AddAccountMethod;

    static TOOL: ProviderDescriptor = ProviderDescriptor {
        id: ProviderId::from_static("tool"),
        display_name: "Tool",
        add_account: &[AddAccountMethod::AutoDetect { reason: "found" }],
        multi_account: false,
        local_usage: false,
    };

    struct Listed(Result<Vec<AccountRef>, ProviderError>);

    #[async_trait]
    impl Provider for Listed {
        fn descriptor(&self) -> &'static ProviderDescriptor {
            &TOOL
        }

        async fn discover(&self) -> Result<Vec<AccountRef>, ProviderError> {
            self.0.clone()
        }

        async fn usage_homes(&self) -> Result<Vec<PathBuf>, ProviderError> {
            Ok(Vec::new())
        }

        async fn fetch_limits(&self, _: &AccountRef) -> Result<LimitsSnapshot, ProviderError> {
            Err(ProviderError::NotSignedIn)
        }

        fn read_usage(
            &self,
            _: &Path,
            _: &mut LogCursors,
        ) -> Result<Vec<UsageEvent>, ProviderError> {
            Ok(Vec::new())
        }
    }

    fn at(home: &str) -> AccountRef {
        AccountRef {
            id: AccountId(format!("tool:{home}")),
            provider: TOOL.id.clone(),
            home: PathBuf::from(home),
            owner: CredentialOwner::Headroom,
        }
    }

    #[tokio::test]
    async fn default_account_at_filters_discovery_by_home() {
        let listed = Listed(Ok(vec![at("/a"), at("/b")]));
        assert_eq!(listed.id().as_str(), "tool");
        assert_eq!(listed.account_at(Path::new("/b")).await, Ok(Some(at("/b"))));
        assert_eq!(listed.account_at(Path::new("/c")).await, Ok(None));
        let signed_out = Listed(Err(ProviderError::NotSignedIn));
        assert_eq!(signed_out.account_at(Path::new("/a")).await, Ok(None));
        let broken = Listed(Err(ProviderError::LocalData("x".into())));
        assert!(broken.account_at(Path::new("/a")).await.is_err());
    }

    #[tokio::test]
    async fn keys_are_unsupported_unless_a_provider_validates_them() {
        let error = Listed(Ok(Vec::new())).validate_key("k").await.unwrap_err();
        assert_eq!(
            error,
            ProviderError::Unsupported("Tool accounts cannot be added with an API key".into())
        );
    }

    #[test]
    fn messages_are_user_safe() {
        assert_eq!(ProviderError::NotSignedIn.to_string(), "not signed in");
        assert_eq!(
            ProviderError::Network("timeout".into()).to_string(),
            "network error: timeout"
        );
        let lapsed = ProviderError::NoSubscription {
            detail: "no active plan".into(),
        };
        assert_eq!(lapsed.to_string(), "no active plan");
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
        let lapsed = ProviderError::NoSubscription {
            detail: "none".into(),
        };
        let json = serde_json::to_string(&lapsed).unwrap();
        assert_eq!(
            serde_json::from_str::<ProviderError>(&json).unwrap(),
            lapsed
        );
    }

    #[test]
    fn a_changed_account_keeps_its_message_and_kind() {
        let changed = ProviderError::AccountChanged("the account at /a has changed".into());
        assert_eq!(changed.to_string(), "the account at /a has changed");
        assert_eq!(
            serde_json::to_string(&changed).unwrap(),
            "{\"kind\":\"account_changed\",\"detail\":\"the account at /a has changed\"}"
        );
        assert!(!changed.needs_sign_in());
    }

    #[test]
    fn sign_in_errors_need_a_new_sign_in() {
        assert!(ProviderError::NotSignedIn.needs_sign_in());
        assert!(ProviderError::SignInExpired.needs_sign_in());
        assert!(ProviderError::ApiKeyOnly.needs_sign_in());
        assert!(!ProviderError::Network("down".into()).needs_sign_in());
    }

    #[test]
    fn only_a_missing_sign_in_means_nothing_to_discover() {
        assert!(ProviderError::NotSignedIn.is_nothing_to_discover());
        assert!(!ProviderError::SignInExpired.is_nothing_to_discover());
        assert!(!ProviderError::Network("down".into()).is_nothing_to_discover());
        assert!(!ProviderError::LocalData("bad".into()).is_nothing_to_discover());
    }
}
