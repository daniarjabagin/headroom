use std::collections::HashMap;
use std::sync::{Mutex, PoisonError};

use headroom_core::account::AccountId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Region {
    Global,
    Mainland,
}

impl Region {
    pub(super) fn currency(self) -> &'static str {
        match self {
            Region::Global => "USD",
            Region::Mainland => "CNY",
        }
    }

    pub(super) fn other(self) -> Region {
        match self {
            Region::Global => Region::Mainland,
            Region::Mainland => Region::Global,
        }
    }
}

#[derive(Debug, Default)]
pub(super) struct KnownRegions(Mutex<HashMap<AccountId, Region>>);

impl KnownRegions {
    pub(super) fn first_try(&self, account: &AccountId) -> Region {
        let known = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        known.get(account).copied().unwrap_or(Region::Global)
    }

    pub(super) fn remember(&self, account: &AccountId, region: Region) {
        let mut known = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        known.insert(account.clone(), region);
    }
}

#[cfg(test)]
mod tests {
    use headroom_core::account::ProviderId;

    use super::*;

    #[test]
    fn regions_start_global_and_are_remembered_per_account() {
        let provider = ProviderId::from_static("moonshot");
        let one = AccountId::from_stable_key(&provider, "one");
        let two = AccountId::from_stable_key(&provider, "two");
        let known = KnownRegions::default();
        assert_eq!(known.first_try(&one), Region::Global);
        known.remember(&one, Region::Mainland);
        assert_eq!(known.first_try(&one), Region::Mainland);
        assert_eq!(known.first_try(&two), Region::Global);
        assert_eq!(Region::Mainland.other(), Region::Global);
        assert_eq!(Region::Global.currency(), "USD");
        assert_eq!(Region::Mainland.currency(), "CNY");
    }
}
