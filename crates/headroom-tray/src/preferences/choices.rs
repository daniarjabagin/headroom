use super::model::{Headline, MAX_REFRESH_SECS, MIN_REFRESH_SECS};
use crate::account::{account_title, shows_name};
use crate::i18n::{Lang, fill};
use crate::payload::{Account, Owner, PanelLimit, State, Status};

const REFRESH_PRESETS: [u32; 7] = [60, 120, 300, 600, 900, 1800, 3600];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Choice<T> {
    pub value: T,
    pub label: String,
}

#[must_use]
pub fn refresh_label(lang: Lang, seconds: u32) -> String {
    let text = seconds.to_string();
    if !seconds.is_multiple_of(60) {
        let forms = ["Every {seconds} second", "Every {seconds} seconds"];
        return fill(
            lang.tr_plural(forms, u64::from(seconds)),
            &[("seconds", &text)],
        );
    }
    let minutes = seconds / 60;
    if minutes == 1 {
        return lang.tr("Every minute").to_owned();
    }
    let forms = ["Every {minutes} minute", "Every {minutes} minutes"];
    fill(
        lang.tr_plural(forms, u64::from(minutes)),
        &[("minutes", &minutes.to_string())],
    )
}

#[must_use]
pub fn refresh_choices(lang: Lang, current: u32) -> Vec<Choice<u32>> {
    let current = current.clamp(MIN_REFRESH_SECS, MAX_REFRESH_SECS);
    let mut values = REFRESH_PRESETS.to_vec();
    if !values.contains(&current) {
        values.push(current);
        values.sort_unstable();
    }
    values
        .into_iter()
        .map(|value| Choice {
            value,
            label: refresh_label(lang, value),
        })
        .collect()
}

fn window_choices(state: Option<&State>) -> Vec<Choice<PanelLimit>> {
    let visible: Vec<&Account> = state
        .map(|state| {
            state
                .accounts
                .iter()
                .filter(|account| !account.hidden)
                .collect()
        })
        .unwrap_or_default();
    let mut choices = Vec::new();
    for account in &visible {
        let title = account_title(account, shows_name(account, &visible));
        choices.extend(account.windows.iter().map(|window| Choice {
            value: PanelLimit {
                account_id: account.id.clone(),
                window: window.id.clone(),
            },
            label: format!("{title} — {}", window.label),
        }));
    }
    choices
}

#[must_use]
pub fn headline_choices(
    lang: Lang,
    state: Option<&State>,
    current: &Headline,
) -> Vec<Choice<Headline>> {
    let mut choices = vec![Choice {
        value: Headline::Auto,
        label: lang.tr("Auto — most critical").to_owned(),
    }];
    choices.extend(window_choices(state).into_iter().map(|choice| Choice {
        value: Headline::Pinned {
            account_id: choice.value.account_id,
            window: choice.value.window,
        },
        label: choice.label,
    }));
    if *current != Headline::Auto && choices.iter().all(|choice| choice.value != *current) {
        choices.push(Choice {
            value: current.clone(),
            label: lang.tr("Pinned limit (not available now)").to_owned(),
        });
    }
    choices
}

#[must_use]
pub fn panel_limit_choices(
    lang: Lang,
    state: Option<&State>,
    current: &[PanelLimit],
) -> Vec<Choice<PanelLimit>> {
    let mut choices = window_choices(state);
    for limit in current {
        if choices.iter().all(|choice| choice.value != *limit) {
            choices.push(Choice {
                value: limit.clone(),
                label: lang.tr("Pinned limit (not available now)").to_owned(),
            });
        }
    }
    choices
}

#[must_use]
pub fn account_name(account: &Account) -> String {
    account
        .label
        .clone()
        .or_else(|| account.email.clone())
        .unwrap_or_else(|| account.provider_name.clone())
}

fn status_text(lang: Lang, status: Status) -> Option<&'static str> {
    match status {
        Status::SignedOut => Some(lang.tr("Signed out")),
        Status::NoSubscription => Some(lang.tr("No active subscription")),
        Status::Error => Some(lang.tr("Refresh failed")),
        Status::Stale => Some(lang.tr("Outdated")),
        Status::Fresh | Status::Refreshing => None,
    }
}

#[must_use]
pub fn account_subtitle(lang: Lang, account: &Account) -> String {
    let mut parts: Vec<&str> = vec![account.provider_name.as_str()];
    parts.extend(account.plan.as_deref());
    if account.label.is_some() {
        parts.extend(account.email.as_deref());
    }
    if account.owner == Owner::Headroom {
        parts.push(lang.tr("added in Headroom"));
    }
    parts.extend(status_text(lang, account.status));
    parts.join(" · ")
}

#[must_use]
pub fn removal_body(lang: Lang, account: &Account) -> String {
    match account.owner {
        Owner::Headroom => lang
            .tr("Headroom deletes the sign-in it created for this account. The account itself is not affected.")
            .to_owned(),
        Owner::Cli => fill(
            lang.tr("Headroom will stop showing this account. The {provider} CLI stays signed in; you can sign in again through Headroom."),
            &[("provider", &account.provider_name)],
        ),
    }
}

#[must_use]
pub fn removal_subtitle(lang: Lang, owner: Owner) -> &'static str {
    match owner {
        Owner::Headroom => lang.tr("Deletes the sign-in Headroom created for this account"),
        Owner::Cli => lang.tr("Stops showing this account. Its CLI stays signed in."),
    }
}

#[must_use]
pub fn moved_order(ids: &[String], id: &str, delta: isize) -> Option<Vec<String>> {
    let from = ids.iter().position(|known| known == id)?;
    let to = from
        .checked_add_signed(delta)
        .filter(|to| *to < ids.len())?;
    if to == from {
        return None;
    }
    let mut order = ids.to_vec();
    let moved = order.remove(from);
    order.insert(to, moved);
    Some(order)
}

#[cfg(test)]
#[path = "choices_tests.rs"]
mod tests;
