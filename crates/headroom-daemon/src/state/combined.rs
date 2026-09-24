use super::combined_pace::combined_pace;
use super::payload::{
    AccountStatus, AccountView, CombinedAccountView, CombinedView, CombinedWindowView, SegmentView,
    WindowView,
};

const MIN_GROUP: usize = 2;

type Member<'a> = (&'a AccountView, &'a WindowView);

struct Group<'a> {
    lead: &'a AccountView,
    members: Vec<&'a AccountView>,
}

#[must_use]
pub fn combined(accounts: &[AccountView], enabled: bool) -> Vec<CombinedView> {
    if !enabled {
        return Vec::new();
    }
    groups(accounts)
        .iter()
        .filter(|group| group.members.len() >= MIN_GROUP)
        .map(group_view)
        .collect()
}

#[must_use]
fn is_combinable(account: &AccountView) -> bool {
    let signed_in = !matches!(
        account.status,
        AccountStatus::SignedOut | AccountStatus::NoSubscription
    );
    !account.hidden && signed_in && visible_windows(account).next().is_some()
}

fn groups(accounts: &[AccountView]) -> Vec<Group<'_>> {
    let mut groups: Vec<Group<'_>> = Vec::new();
    for account in accounts.iter().filter(|a| is_combinable(a)) {
        match groups
            .iter_mut()
            .find(|g| g.lead.provider == account.provider)
        {
            Some(group) => group.members.push(account),
            None => groups.push(Group {
                lead: account,
                members: vec![account],
            }),
        }
    }
    groups
}

fn visible_windows(account: &AccountView) -> impl Iterator<Item = &WindowView> {
    account.windows.iter().filter(|w| !w.hidden)
}

fn group_view(group: &Group<'_>) -> CombinedView {
    let members = &group.members;
    CombinedView {
        provider: group.lead.provider.clone(),
        provider_name: group.lead.provider_name.clone(),
        account_ids: members.iter().map(|a| a.id.clone()).collect(),
        accounts: members.iter().map(|a| account_entry(a)).collect(),
        windows: first_windows(members)
            .into_iter()
            .map(|lead| combined_window(lead, &segments_of(members, &lead.id)))
            .collect(),
    }
}

fn account_entry(account: &AccountView) -> CombinedAccountView {
    CombinedAccountView {
        account_id: account.id.clone(),
        label: account.display_label(),
        plan: account.plan.clone(),
    }
}

fn first_windows<'a>(members: &[&'a AccountView]) -> Vec<&'a WindowView> {
    let mut firsts: Vec<&WindowView> = Vec::new();
    for window in members.iter().flat_map(|a| visible_windows(a)) {
        if !firsts.iter().any(|w| w.id == window.id) {
            firsts.push(window);
        }
    }
    firsts
}

fn segments_of<'a>(members: &[&'a AccountView], id: &str) -> Vec<Member<'a>> {
    members
        .iter()
        .filter_map(|account| {
            visible_windows(account)
                .find(|w| w.id == id)
                .map(|w| (*account, w))
        })
        .collect()
}

fn combined_window(lead: &WindowView, segments: &[Member<'_>]) -> CombinedWindowView {
    let windows: Vec<&WindowView> = segments.iter().map(|(_, w)| *w).collect();
    let (pace, tone) = combined_pace(&windows);
    CombinedWindowView {
        id: lead.id.clone(),
        label: lead.label.clone(),
        capacity_percent: capacity_percent(windows.len()),
        remaining_percent: windows.iter().map(|w| w.remaining_percent).sum(),
        used_percent: windows.iter().map(|w| w.used_percent).sum(),
        resets_at: windows.iter().filter_map(|w| w.resets_at).min(),
        tone,
        pace,
        segments: segments.iter().map(|m| segment(*m)).collect(),
    }
}

fn capacity_percent(accounts: usize) -> u32 {
    u32::try_from(accounts)
        .unwrap_or(u32::MAX)
        .saturating_mul(100)
}

fn segment((account, window): Member<'_>) -> SegmentView {
    SegmentView {
        account_id: account.id.clone(),
        label: account.display_label(),
        remaining_percent: window.remaining_percent,
        used_percent: window.used_percent,
        resets_at: window.resets_at,
        tone: window.tone,
    }
}

#[cfg(test)]
#[path = "combined_tests.rs"]
mod tests;
