use super::payload::{AccountView, CombinedView};
use crate::settings::DisplaySettings;

#[must_use]
pub fn account_collapsed(account: &AccountView, display: &DisplaySettings) -> bool {
    display.collapse_unstarred && !is_starred(display, &account.id)
}

#[must_use]
pub fn with_group_collapse(
    combined: Vec<CombinedView>,
    display: &DisplaySettings,
) -> Vec<CombinedView> {
    combined
        .into_iter()
        .map(|group| CombinedView {
            collapsed: group_collapsed(&group, display),
            ..group
        })
        .collect()
}

fn group_collapsed(group: &CombinedView, display: &DisplaySettings) -> bool {
    display.collapse_unstarred && !group.account_ids.iter().any(|id| is_starred(display, id))
}

fn is_starred(display: &DisplaySettings, account_id: &str) -> bool {
    display.starred_accounts.iter().any(|id| id == account_id)
}

#[cfg(test)]
#[path = "account_collapse_tests.rs"]
mod tests;
