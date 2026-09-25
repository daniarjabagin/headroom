use headroom_core::pace::Tone;

use super::payload::{
    AccountStatus, AccountView, CombinedView, CombinedWindowView, Headline, WindowView,
};
use crate::settings::HeadlineMode;

#[derive(Debug, Clone, Copy)]
pub(super) enum Candidate<'a> {
    Single(&'a AccountView, &'a WindowView),
    Combined(&'a CombinedView, &'a CombinedWindowView),
}

impl Candidate<'_> {
    pub(super) fn tone(self) -> Tone {
        match self {
            Candidate::Single(_, w) => w.tone,
            Candidate::Combined(_, w) => w.tone,
        }
    }

    pub(super) fn remaining_percent(self) -> f64 {
        match self {
            Candidate::Single(_, w) => w.remaining_percent,
            Candidate::Combined(_, w) => share(w, w.remaining_percent),
        }
    }

    pub(super) fn is(self, account_id: &str, window: &str) -> bool {
        match self {
            Candidate::Single(a, w) => a.id == account_id && w.id == window,
            Candidate::Combined(g, w) => {
                g.account_ids.iter().any(|id| id == account_id) && w.id == window
            }
        }
    }

    fn more_critical_than(self, current: Candidate<'_>) -> bool {
        self.tone() > current.tone()
            || (self.tone() == current.tone()
                && self.remaining_percent() < current.remaining_percent())
    }

    pub(super) fn headline(self) -> Headline {
        match self {
            Candidate::Single(account, window) => single_headline(account, window),
            Candidate::Combined(group, window) => combined_headline(group, window),
        }
    }
}

#[must_use]
pub fn headline(
    accounts: &[AccountView],
    combined: &[CombinedView],
    mode: &HeadlineMode,
) -> Option<Headline> {
    select(&candidates(accounts, combined), mode).map(Candidate::headline)
}

pub(super) fn select<'a>(
    candidates: &[Candidate<'a>],
    mode: &HeadlineMode,
) -> Option<Candidate<'a>> {
    pinned(candidates, mode).or_else(|| most_critical(candidates))
}

fn pinned<'a>(candidates: &[Candidate<'a>], mode: &HeadlineMode) -> Option<Candidate<'a>> {
    let HeadlineMode::Pinned { account_id, window } = mode else {
        return None;
    };
    candidates
        .iter()
        .copied()
        .find(|c| c.is(account_id, window))
}

fn most_critical<'a>(candidates: &[Candidate<'a>]) -> Option<Candidate<'a>> {
    candidates
        .iter()
        .copied()
        .fold(None, |best, candidate| match best {
            Some(current) if !candidate.more_critical_than(current) => Some(current),
            _ => Some(candidate),
        })
}

pub(super) fn candidates<'a>(
    accounts: &'a [AccountView],
    combined: &'a [CombinedView],
) -> Vec<Candidate<'a>> {
    let mut candidates = Vec::new();
    let listed = accounts
        .iter()
        .filter(|a| !a.hidden && a.status != AccountStatus::NoSubscription);
    for account in listed {
        match combined
            .iter()
            .find(|g| g.account_ids.contains(&account.id))
        {
            Some(group) if leads(group, account) => {
                candidates.extend(group.windows.iter().map(|w| Candidate::Combined(group, w)));
            }
            Some(_) => {}
            None => candidates.extend(
                account
                    .windows
                    .iter()
                    .filter(|w| !w.hidden)
                    .map(|w| Candidate::Single(account, w)),
            ),
        }
    }
    candidates
}

fn leads(group: &CombinedView, account: &AccountView) -> bool {
    group.account_ids.first() == Some(&account.id)
}

pub(super) fn share(window: &CombinedWindowView, value: f64) -> f64 {
    value / f64::from(window.capacity_percent) * 100.0
}

fn single_headline(account: &AccountView, window: &WindowView) -> Headline {
    Headline {
        account_id: account.id.clone(),
        provider: account.provider.clone(),
        provider_name: account.provider_name.clone(),
        account_label: Some(account.display_label()),
        window: window.id.clone(),
        window_label: window.label.clone(),
        used_percent: window.used_percent,
        remaining_percent: window.remaining_percent,
        tone: window.tone,
        combined: false,
        account_count: 1,
    }
}

fn combined_headline(group: &CombinedView, window: &CombinedWindowView) -> Headline {
    Headline {
        account_id: window
            .segments
            .first()
            .map(|s| s.account_id.clone())
            .unwrap_or_default(),
        provider: group.provider.clone(),
        provider_name: group.provider_name.clone(),
        account_label: None,
        window: window.id.clone(),
        window_label: window.label.clone(),
        used_percent: share(window, window.used_percent),
        remaining_percent: share(window, window.remaining_percent),
        tone: window.tone,
        combined: true,
        account_count: window.segments.len(),
    }
}

#[cfg(test)]
#[path = "headline_tests.rs"]
mod tests;
