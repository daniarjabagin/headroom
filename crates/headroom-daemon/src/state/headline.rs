use super::payload::{AccountStatus, AccountView, Headline, WindowView};
use crate::settings::HeadlineMode;

#[must_use]
pub fn headline(accounts: &[AccountView], mode: &HeadlineMode) -> Option<Headline> {
    pinned(accounts, mode).or_else(|| most_critical(accounts))
}

fn pinned(accounts: &[AccountView], mode: &HeadlineMode) -> Option<Headline> {
    let HeadlineMode::Pinned { account_id, window } = mode else {
        return None;
    };
    candidates(accounts)
        .find(|(account, w)| &account.id == account_id && &w.id == window)
        .map(|(account, w)| to_headline(account, w))
}

fn most_critical(accounts: &[AccountView]) -> Option<Headline> {
    candidates(accounts)
        .fold(
            None,
            |best: Option<(&AccountView, &WindowView)>, candidate| match best {
                Some(current) if !more_critical(candidate.1, current.1) => Some(current),
                _ => Some(candidate),
            },
        )
        .map(|(account, w)| to_headline(account, w))
}

fn candidates(accounts: &[AccountView]) -> impl Iterator<Item = (&AccountView, &WindowView)> {
    accounts
        .iter()
        .filter(|a| !a.hidden && a.status != AccountStatus::NoSubscription)
        .flat_map(|account| account.windows.iter().map(move |w| (account, w)))
        .filter(|(_, w)| !w.hidden)
}

fn more_critical(candidate: &WindowView, current: &WindowView) -> bool {
    candidate.tone > current.tone
        || (candidate.tone == current.tone
            && candidate.remaining_percent < current.remaining_percent)
}

fn to_headline(account: &AccountView, window: &WindowView) -> Headline {
    Headline {
        account_id: account.id.clone(),
        provider: account.provider,
        account_label: account_label(account),
        window: window.id.clone(),
        window_label: window.label.clone(),
        used_percent: window.used_percent,
        remaining_percent: window.remaining_percent,
        tone: window.tone,
    }
}

fn account_label(account: &AccountView) -> String {
    account
        .label
        .clone()
        .or_else(|| account.email.clone())
        .unwrap_or_else(|| account.provider.display_name().to_owned())
}

#[cfg(test)]
#[path = "headline_tests.rs"]
mod tests;
