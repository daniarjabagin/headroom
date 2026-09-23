use std::fmt;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderKind {
    Codex,
    Claude,
}

impl ProviderKind {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            ProviderKind::Codex => "codex",
            ProviderKind::Claude => "claude",
        }
    }

    #[must_use]
    pub fn display_name(self) -> &'static str {
        match self {
            ProviderKind::Codex => "Codex",
            ProviderKind::Claude => "Claude Code",
        }
    }
}

impl fmt::Display for ProviderKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AccountId(pub String);

impl AccountId {
    const HASH_HEX_LEN: usize = 12;

    #[must_use]
    pub fn from_stable_key(provider: ProviderKind, stable_key: &str) -> AccountId {
        let digest = hex::encode(Sha256::digest(stable_key.as_bytes()));
        let short = digest.get(..Self::HASH_HEX_LEN).unwrap_or(&digest);
        AccountId(format!("{provider}:{short}"))
    }
}

impl fmt::Display for AccountId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CredentialOwner {
    Cli,
    Headroom,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountRef {
    pub id: AccountId,
    pub provider: ProviderKind,
    pub home: PathBuf,
    pub owner: CredentialOwner,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountIdentity {
    pub email: Option<String>,
    pub plan: Option<String>,
    pub stable_key: String,
}

impl AccountIdentity {
    #[must_use]
    pub fn account_id(&self, provider: ProviderKind) -> AccountId {
        AccountId::from_stable_key(provider, &self.stable_key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn account_id_uses_provider_prefix_and_short_hash() {
        let id = AccountId::from_stable_key(ProviderKind::Codex, "user-1/account-1");
        let expected = hex::encode(Sha256::digest(b"user-1/account-1"));
        assert_eq!(id.0, format!("codex:{}", &expected[..12]));
    }

    #[test]
    fn account_id_is_stable_and_provider_specific() {
        let codex = AccountId::from_stable_key(ProviderKind::Codex, "k");
        let claude = AccountId::from_stable_key(ProviderKind::Claude, "k");
        assert_eq!(codex, AccountId::from_stable_key(ProviderKind::Codex, "k"));
        assert_eq!(codex.0[6..], claude.0[7..]);
        assert!(claude.0.starts_with("claude:"));
    }

    #[test]
    fn identity_derives_account_id() {
        let identity = AccountIdentity {
            email: None,
            plan: Some("Pro".into()),
            stable_key: "a/o".into(),
        };
        assert_eq!(
            identity.account_id(ProviderKind::Claude),
            AccountId::from_stable_key(ProviderKind::Claude, "a/o")
        );
    }

    #[test]
    fn enums_serialize_snake_case() {
        assert_eq!(
            serde_json::to_string(&ProviderKind::Claude).unwrap(),
            "\"claude\""
        );
        assert_eq!(
            serde_json::to_string(&CredentialOwner::Headroom).unwrap(),
            "\"headroom\""
        );
    }
}
