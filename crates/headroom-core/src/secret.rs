use std::fmt;

use async_trait::async_trait;

use crate::account::AccountId;
use crate::provider::ProviderError;

#[derive(Clone, PartialEq, Eq)]
pub struct SecretString(String);

impl SecretString {
    #[must_use]
    pub fn new(value: String) -> SecretString {
        SecretString(value)
    }

    #[must_use]
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for SecretString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SecretString([redacted])")
    }
}

/// Stored API keys, looked up by the account they were saved for.
#[async_trait]
pub trait SecretReader: Send + Sync {
    async fn read_secret(&self, account: &AccountId)
    -> Result<Option<SecretString>, ProviderError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_output_never_shows_the_value() {
        let secret = SecretString::new("sk-live-123".into());
        assert_eq!(format!("{secret:?}"), "SecretString([redacted])");
        assert_eq!(secret.expose(), "sk-live-123");
    }
}
