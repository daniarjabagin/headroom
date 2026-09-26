use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

use headroom_core::account::{AccountRef, CredentialOwner, ProviderId};
use jiff::Timestamp;

use crate::dismissed::DismissedHomes;
use crate::home::UsageHome;
use crate::model::Model;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CliSignIns(BTreeMap<ProviderId, Vec<AccountRef>>);

impl CliSignIns {
    pub fn set(&mut self, provider: &ProviderId, found: &[AccountRef]) {
        let cli = found
            .iter()
            .filter(|a| &a.provider == provider && a.owner == CredentialOwner::Cli)
            .cloned()
            .collect();
        self.0.insert(provider.clone(), cli);
    }

    #[must_use]
    pub fn from_dismissed(dismissed: &DismissedHomes) -> CliSignIns {
        let mut by_provider: BTreeMap<ProviderId, Vec<AccountRef>> = BTreeMap::new();
        for home in dismissed.iter() {
            by_provider
                .entry(home.provider.clone())
                .or_default()
                .push(AccountRef {
                    id: home.account.clone(),
                    provider: home.provider.clone(),
                    home: home.home.clone(),
                    owner: CredentialOwner::Cli,
                });
        }
        CliSignIns(by_provider)
    }

    fn of(&self, provider: &ProviderId) -> &[AccountRef] {
        self.0.get(provider).map_or(&[], Vec::as_slice)
    }
}

#[must_use]
pub fn linked_homes<'a>(
    sign_ins: &'a [AccountRef],
    visible: &[&AccountRef],
    account: &AccountRef,
) -> Vec<&'a Path> {
    if account.owner != CredentialOwner::Headroom {
        return Vec::new();
    }
    sign_ins
        .iter()
        .filter(|cli| same_identity(cli, account) && cli.home != account.home)
        .filter(|cli| !occupied(visible, cli))
        .map(|cli| cli.home.as_path())
        .collect()
}

fn same_identity(cli: &AccountRef, account: &AccountRef) -> bool {
    cli.provider == account.provider && cli.id == account.id
}

fn occupied(visible: &[&AccountRef], cli: &AccountRef) -> bool {
    visible
        .iter()
        .any(|shown| shown.provider == cli.provider && shown.home == cli.home)
}

impl Model {
    #[must_use]
    pub fn linked_log_homes(&self, account: &AccountRef) -> Vec<PathBuf> {
        let visible: Vec<&AccountRef> = self.active_accounts().map(|a| &a.reference).collect();
        let sign_ins = self.cli_sign_ins.of(&account.provider);
        linked_homes(sign_ins, &visible, account)
            .into_iter()
            .map(Path::to_path_buf)
            .collect()
    }

    #[must_use]
    pub fn usage_home_of(&self, account: &AccountRef) -> PathBuf {
        self.linked_log_homes(account)
            .into_iter()
            .find(|home| self.lists_usage(&usage_home(account, home)))
            .unwrap_or_else(|| account.home.clone())
    }

    #[must_use]
    pub fn absorbed_homes(&self) -> HashMap<UsageHome, UsageHome> {
        let mut absorbed = HashMap::new();
        for account in self.active_accounts().map(|record| &record.reference) {
            let primary = usage_home(account, &self.usage_home_of(account));
            let members = std::iter::once(account.home.clone())
                .chain(self.linked_log_homes(account))
                .map(|home| usage_home(account, &home))
                .filter(|home| home != &primary && self.lists_usage(home));
            for member in members {
                absorbed.insert(member, primary.clone());
            }
        }
        absorbed
    }

    fn lists_usage(&self, home: &UsageHome) -> bool {
        self.usage_homes.contains(home) && self.usage.contains_key(home)
    }

    #[must_use]
    pub fn account_is_live(&self, account: &AccountRef, now: Timestamp) -> bool {
        self.activity.any_live(&self.log_homes_of(account), now)
    }

    #[must_use]
    pub fn has_activity_source(&self, account: &AccountRef) -> bool {
        self.log_homes_of(account)
            .iter()
            .any(|home| self.usage_homes.contains(home))
    }

    fn log_homes_of(&self, account: &AccountRef) -> Vec<UsageHome> {
        std::iter::once(account.home.clone())
            .chain(self.linked_log_homes(account))
            .map(|home| usage_home(account, &home))
            .collect()
    }
}

fn usage_home(account: &AccountRef, home: &Path) -> UsageHome {
    UsageHome {
        provider: account.provider.clone(),
        home: home.to_path_buf(),
    }
}

#[cfg(test)]
#[path = "log_homes_tests.rs"]
mod tests;
