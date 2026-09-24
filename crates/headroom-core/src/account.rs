use std::borrow::Cow;
use std::collections::HashSet;
use std::fmt;
use std::path::PathBuf;

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ProviderId(Cow<'static, str>);

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("invalid provider id {0:?}: use lowercase letters, digits, '-' or '_'")]
pub struct InvalidProviderId(pub String);

impl ProviderId {
    /// Wraps a literal without checking it; registry tests check every literal id.
    #[must_use]
    pub const fn from_static(id: &'static str) -> ProviderId {
        ProviderId(Cow::Borrowed(id))
    }

    pub fn parse(text: &str) -> Result<ProviderId, InvalidProviderId> {
        if is_valid_id(text) {
            Ok(ProviderId(Cow::Owned(text.to_owned())))
        } else {
            Err(InvalidProviderId(text.to_owned()))
        }
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    #[must_use]
    pub fn is_valid(&self) -> bool {
        is_valid_id(&self.0)
    }
}

fn is_valid_id(text: &str) -> bool {
    !text.is_empty()
        && text
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-' || b == b'_')
}

impl fmt::Display for ProviderId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl Serialize for ProviderId {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for ProviderId {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<ProviderId, D::Error> {
        let text = String::deserialize(deserializer)?;
        ProviderId::parse(&text).map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AccountId(pub String);

impl AccountId {
    const HASH_HEX_LEN: usize = 12;

    #[must_use]
    pub fn from_stable_key(provider: &ProviderId, stable_key: &str) -> AccountId {
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
    pub provider: ProviderId,
    pub home: PathBuf,
    pub owner: CredentialOwner,
}

/// Keeps the first record of every account id, so records listed earlier win.
#[must_use]
pub fn first_per_id(accounts: Vec<AccountRef>) -> Vec<AccountRef> {
    let mut seen = HashSet::new();
    accounts
        .into_iter()
        .filter(|account| seen.insert(account.id.clone()))
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountIdentity {
    pub email: Option<String>,
    pub plan: Option<String>,
    pub stable_key: String,
}

impl AccountIdentity {
    #[must_use]
    pub fn account_id(&self, provider: &ProviderId) -> AccountId {
        AccountId::from_stable_key(provider, &self.stable_key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CODEX: ProviderId = ProviderId::from_static("codex");
    const CLAUDE: ProviderId = ProviderId::from_static("claude");

    #[test]
    fn account_id_uses_provider_prefix_and_short_hash() {
        let id = AccountId::from_stable_key(&CODEX, "user-1/account-1");
        let expected = hex::encode(Sha256::digest(b"user-1/account-1"));
        assert_eq!(id.0, format!("codex:{}", &expected[..12]));
    }

    #[test]
    fn account_id_is_stable_and_provider_specific() {
        let codex = AccountId::from_stable_key(&CODEX, "k");
        let claude = AccountId::from_stable_key(&CLAUDE, "k");
        assert_eq!(codex, AccountId::from_stable_key(&CODEX, "k"));
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
            identity.account_id(&CLAUDE),
            AccountId::from_stable_key(&CLAUDE, "a/o")
        );
    }

    fn at(id: &str, home: &str, owner: CredentialOwner) -> AccountRef {
        AccountRef {
            id: AccountId(id.into()),
            provider: CODEX,
            home: PathBuf::from(home),
            owner,
        }
    }

    #[test]
    fn first_record_of_each_id_wins() {
        let cli = at("codex:a", "/cli", CredentialOwner::Cli);
        let own = at("codex:a", "/own", CredentialOwner::Headroom);
        let other = at("codex:b", "/b", CredentialOwner::Headroom);
        let kept = first_per_id(vec![cli.clone(), other.clone(), own.clone()]);
        assert_eq!(kept, [cli, other.clone()]);
        assert_eq!(first_per_id(vec![own.clone(), other.clone()]), [own, other]);
    }

    #[test]
    fn ids_and_owners_serialize_as_strings() {
        assert_eq!(serde_json::to_string(&CLAUDE).unwrap(), "\"claude\"");
        assert_eq!(
            serde_json::to_string(&CredentialOwner::Headroom).unwrap(),
            "\"headroom\""
        );
    }

    #[test]
    fn provider_ids_accept_only_lowercase_ascii_words() {
        for good in ["codex", "open-code", "z_ai", "kimi2"] {
            assert_eq!(ProviderId::parse(good).unwrap().as_str(), good);
        }
        for bad in ["", "Codex", "open code", "../x", "é"] {
            assert!(ProviderId::parse(bad).is_err(), "{bad}");
        }
        assert!(CODEX.is_valid());
        assert!(!ProviderId::from_static("Bad").is_valid());
    }

    #[test]
    fn static_and_parsed_ids_are_equal_and_round_trip() {
        let parsed = ProviderId::parse("codex").unwrap();
        assert_eq!(parsed, CODEX);
        let json = serde_json::to_string(&parsed).unwrap();
        assert_eq!(serde_json::from_str::<ProviderId>(&json).unwrap(), CODEX);
        assert!(serde_json::from_str::<ProviderId>("\"No Way\"").is_err());
    }
}
