use super::payload::{AccountView, Headline, WindowView};
use crate::settings::HeadlineMode;

#[must_use]
pub fn headline(accounts: &[AccountView], mode: &HeadlineMode) -> Option<Headline> {
    pinned(accounts, mode).or_else(|| most_critical(accounts))
}

fn pinned(accounts: &[AccountView], mode: &HeadlineMode) -> Option<Headline> {
    let HeadlineMode::Pinned { account_id, window } = mode else {
        return None;
    };
    visible(accounts)
        .filter(|account| &account.id == account_id)
        .flat_map(|account| account.windows.iter().map(move |w| (account, w)))
        .find(|(_, w)| &w.id == window)
        .map(|(account, w)| to_headline(account, w))
}

fn most_critical(accounts: &[AccountView]) -> Option<Headline> {
    visible(accounts)
        .flat_map(|account| account.windows.iter().map(move |w| (account, w)))
        .fold(
            None,
            |best: Option<(&AccountView, &WindowView)>, candidate| match best {
                Some(current) if !more_critical(candidate.1, current.1) => Some(current),
                _ => Some(candidate),
            },
        )
        .map(|(account, w)| to_headline(account, w))
}

fn visible(accounts: &[AccountView]) -> impl Iterator<Item = &AccountView> {
    accounts.iter().filter(|a| !a.hidden)
}

fn more_critical(candidate: &WindowView, current: &WindowView) -> bool {
    candidate.tone > current.tone
        || (candidate.tone == current.tone
            && candidate.remaining_percent < current.remaining_percent)
}

fn to_headline(account: &AccountView, window: &WindowView) -> Headline {
    Headline {
        account_id: account.id.clone(),
        window: window.id.clone(),
        remaining_percent: window.remaining_percent,
        tone: window.tone,
    }
}
