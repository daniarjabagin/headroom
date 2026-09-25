use headroom_core::pace::Tone;

use super::payload::{AccountStatus, AccountView, CombinedView};
use crate::settings::DisplaySettings;

#[must_use]
pub fn account_collapsed(account: &AccountView, display: &DisplaySettings) -> bool {
    display.collapse_unstarred
        && !is_starred(display, &account.id)
        && is_healthy(account)
        && calm(account.windows.iter().filter(|w| !w.hidden).map(|w| w.tone))
}

#[must_use]
pub fn with_group_collapse(
    combined: Vec<CombinedView>,
    accounts: &[AccountView],
    display: &DisplaySettings,
) -> Vec<CombinedView> {
    combined
        .into_iter()
        .map(|group| CombinedView {
            collapsed: group_collapsed(&group, accounts, display),
            ..group
        })
        .collect()
}

fn group_collapsed(
    group: &CombinedView,
    accounts: &[AccountView],
    display: &DisplaySettings,
) -> bool {
    display.collapse_unstarred
        && !group.account_ids.iter().any(|id| is_starred(display, id))
        && members(group, accounts).all(is_healthy)
        && calm(group.windows.iter().map(|w| w.tone))
}

fn members<'a>(
    group: &'a CombinedView,
    accounts: &'a [AccountView],
) -> impl Iterator<Item = &'a AccountView> {
    accounts
        .iter()
        .filter(|account| group.account_ids.contains(&account.id))
}

fn is_healthy(account: &AccountView) -> bool {
    let troubled = matches!(
        account.status,
        AccountStatus::SignedOut | AccountStatus::Error | AccountStatus::NoSubscription
    );
    account.error.is_none() && !troubled
}

fn is_starred(display: &DisplaySettings, account_id: &str) -> bool {
    display.starred_accounts.iter().any(|id| id == account_id)
}

fn calm(mut tones: impl Iterator<Item = Tone>) -> bool {
    tones.all(|tone| tone < Tone::Warning)
}

#[cfg(test)]
#[path = "account_collapse_tests.rs"]
mod tests;
