use std::collections::BTreeSet;
use std::path::PathBuf;

use headroom_core::account::{AccountId, AccountRef, CredentialOwner, ProviderId, first_per_id};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct DismissedHome {
    pub provider: ProviderId,
    pub account: AccountId,
    pub home: PathBuf,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DismissedHomes(BTreeSet<DismissedHome>);

impl DismissedHome {
    #[must_use]
    pub fn of(account: &AccountRef) -> DismissedHome {
        DismissedHome {
            provider: account.provider.clone(),
            account: account.id.clone(),
            home: account.home.clone(),
        }
    }
}

impl DismissedHomes {
    #[must_use]
    pub fn hides(&self, account: &AccountRef) -> bool {
        account.owner == CredentialOwner::Cli && self.0.contains(&DismissedHome::of(account))
    }

    pub fn insert(&mut self, home: DismissedHome) {
        self.0.insert(home);
    }

    pub fn restore(&mut self, provider: Option<&ProviderId>) {
        match provider {
            None => self.0.clear(),
            Some(provider) => self.0.retain(|home| &home.provider != provider),
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &DismissedHome> {
        self.0.iter()
    }

    #[must_use]
    pub fn resolve(&self, found: Vec<AccountRef>) -> Vec<AccountRef> {
        let kept = found.into_iter().filter(|a| !self.hides(a)).collect();
        first_per_id(kept)
    }
}

impl FromIterator<DismissedHome> for DismissedHomes {
    fn from_iter<I: IntoIterator<Item = DismissedHome>>(homes: I) -> DismissedHomes {
        DismissedHomes(homes.into_iter().collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{CLAUDE, CODEX};

    fn at(id: &str, home: &str, owner: CredentialOwner) -> AccountRef {
        AccountRef {
            id: AccountId(format!("codex:{id}")),
            provider: CODEX,
            home: PathBuf::from(home),
            owner,
        }
    }

    fn cli() -> AccountRef {
        at("ada", "/home/ada/.codex", CredentialOwner::Cli)
    }

    fn own() -> AccountRef {
        at(
            "ada",
            "/data/headroom/accounts/codex/1",
            CredentialOwner::Headroom,
        )
    }

    fn dismissed(accounts: &[AccountRef]) -> DismissedHomes {
        accounts.iter().map(DismissedHome::of).collect()
    }

    #[test]
    fn without_dismissals_the_first_home_of_an_id_wins() {
        let resolved = DismissedHomes::default().resolve(vec![cli(), own()]);
        assert_eq!(resolved, [cli()]);
    }

    #[test]
    fn a_dismissed_cli_home_gives_way_to_the_headroom_home() {
        let resolved = dismissed(&[cli()]).resolve(vec![cli(), own()]);
        assert_eq!(resolved, [own()]);
    }

    #[test]
    fn a_dismissed_cli_home_alone_is_hidden() {
        assert!(dismissed(&[cli()]).resolve(vec![cli()]).is_empty());
    }

    #[test]
    fn headroom_homes_are_never_hidden() {
        let homes = dismissed(&[own()]);
        assert!(!homes.hides(&own()));
        assert_eq!(homes.resolve(vec![own()]), [own()]);
    }

    #[test]
    fn only_the_dismissed_account_of_a_shared_cli_home_is_hidden() {
        let other = at("bob", "/home/ada/.codex", CredentialOwner::Cli);
        let resolved = dismissed(&[cli()]).resolve(vec![cli(), other.clone()]);
        assert_eq!(resolved, [other]);
    }

    #[test]
    fn restoring_a_provider_keeps_the_others() {
        let claude = AccountRef {
            provider: CLAUDE,
            ..at("x", "/home/ada/.claude", CredentialOwner::Cli)
        };
        let mut homes = dismissed(&[cli(), claude.clone()]);
        homes.restore(Some(&CODEX));
        assert!(!homes.hides(&cli()));
        assert!(homes.hides(&claude));
        homes.restore(None);
        assert_eq!(homes, DismissedHomes::default());
    }
}
