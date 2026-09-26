use super::choices::{account_name, moved_order, status_text};
use super::registry::ProviderInfo;
use crate::i18n::Lang;
use crate::payload::{Account, Display, ProviderStatus, Recovery, RecoveryField, Status, Tone};
use crate::popup_model::links::{LinkKind, quick_links};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mark {
    Tone(Tone),
    Problem,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SidebarItem {
    pub account_id: String,
    pub provider: String,
    pub title: String,
    pub subtitle: String,
    pub mark: Mark,
    pub note: Option<&'static str>,
    pub starred: bool,
    pub hidden: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Moves {
    pub up: bool,
    pub down: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkItem {
    pub kind: LinkKind,
    pub title: &'static str,
    pub subtitle: String,
    pub url: String,
    pub alert: bool,
}

fn tone_rank(tone: Tone) -> u8 {
    match tone {
        Tone::Critical => 3,
        Tone::Warning => 2,
        Tone::Good => 1,
        Tone::Neutral => 0,
    }
}

fn has_problem(status: Status) -> bool {
    matches!(
        status,
        Status::Error | Status::SignedOut | Status::NoSubscription
    )
}

#[must_use]
pub fn account_mark(account: &Account, display: &Display) -> Mark {
    if has_problem(account.status) {
        return Mark::Problem;
    }
    let worst = account
        .windows
        .iter()
        .filter(|window| !display.is_window_hidden(&account.id, &window.id))
        .map(|window| window.tone)
        .max_by_key(|tone| tone_rank(*tone));
    Mark::Tone(worst.unwrap_or(Tone::Neutral))
}

fn names_account(account: &Account, accounts: &[Account]) -> bool {
    let same_provider = accounts
        .iter()
        .filter(|other| other.provider == account.provider)
        .count();
    same_provider > 1 || account.label.is_some()
}

#[must_use]
pub fn sidebar_title(account: &Account, accounts: &[Account]) -> String {
    if names_account(account, accounts) {
        format!("{} · {}", account.provider_name, account_name(account))
    } else {
        account.provider_name.clone()
    }
}

fn sidebar_subtitle(account: &Account, named: bool) -> String {
    let email_in_title = named && account.label.is_none();
    let mut parts: Vec<&str> = account.plan.as_deref().into_iter().collect();
    if !email_in_title {
        parts.extend(account.email.as_deref());
    }
    parts.join(" · ")
}

#[must_use]
pub fn sidebar_items(lang: Lang, accounts: &[Account], display: &Display) -> Vec<SidebarItem> {
    accounts
        .iter()
        .map(|account| SidebarItem {
            account_id: account.id.clone(),
            provider: account.provider.clone(),
            title: sidebar_title(account, accounts),
            subtitle: sidebar_subtitle(account, names_account(account, accounts)),
            mark: account_mark(account, display),
            note: status_text(lang, account.status),
            starred: display.is_starred(&account.id),
            hidden: account.hidden,
        })
        .collect()
}

#[must_use]
pub fn kept_selection(
    items: &[SidebarItem],
    previous: &[String],
    current: Option<&str>,
) -> Option<String> {
    if let Some(current) = current.filter(|id| items.iter().any(|item| item.account_id == *id)) {
        return Some(current.to_owned());
    }
    let place = current
        .and_then(|id| previous.iter().position(|known| known == id))
        .unwrap_or(0);
    items
        .get(place.min(items.len().saturating_sub(1)))
        .map(|item| item.account_id.clone())
}

#[must_use]
pub fn account_moves(order: &[String], account_id: &str) -> Moves {
    Moves {
        up: moved_order(order, account_id, -1).is_some(),
        down: moved_order(order, account_id, 1).is_some(),
    }
}

#[must_use]
pub fn offers_sign_in(account: &Account) -> bool {
    let offered = matches!(
        account.recovery,
        RecoveryField::Offered(Recovery::SignIn { .. } | Recovery::CliLogin { .. })
    );
    offered || account.status == Status::SignedOut
}

fn link_subtitle(kind: LinkKind, host: &str, status: Option<&ProviderStatus>) -> (String, bool) {
    let incident = status
        .filter(|status| kind == LinkKind::Status && !status.is_clear())
        .and_then(|status| status.title.clone());
    match incident {
        Some(title) => (format!("{title} · {host}"), true),
        None => (host.to_owned(), false),
    }
}

#[must_use]
pub fn account_links(
    lang: Lang,
    provider: Option<&ProviderInfo>,
    status: Option<&ProviderStatus>,
) -> Vec<LinkItem> {
    let Some(provider) = provider else {
        return Vec::new();
    };
    quick_links(&provider.links)
        .into_iter()
        .map(|link| {
            let (subtitle, alert) = link_subtitle(link.kind, &link.host, status);
            LinkItem {
                kind: link.kind,
                title: link.kind.title(lang),
                subtitle,
                url: link.url,
                alert,
            }
        })
        .collect()
}

#[cfg(test)]
#[path = "accounts_tests.rs"]
mod tests;
