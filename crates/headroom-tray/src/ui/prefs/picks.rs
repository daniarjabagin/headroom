use crate::format::{percent_reading, window_label};
use crate::i18n::{Lang, fill};
use crate::payload::{Account, Display, PanelLimit, State, ValueMode};
use crate::preferences::choices::account_name;
use crate::preferences::display::MAX_PANEL_LIMITS;
use crate::preferences::model::Notifications;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LimitItem {
    pub limit: PanelLimit,
    pub provider: String,
    pub title: String,
    pub subtitle: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StarItem {
    pub account_id: String,
    pub provider: String,
    pub title: String,
    pub subtitle: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderItem {
    pub id: String,
    pub name: String,
}

fn visible_accounts(state: &State) -> impl Iterator<Item = &Account> {
    state.accounts.iter().filter(|account| !account.hidden)
}

fn account_limits(lang: Lang, display: &Display, account: &Account) -> Vec<LimitItem> {
    let who = account_name(account);
    account
        .windows
        .iter()
        .filter(|window| !display.is_window_hidden(&account.id, &window.id))
        .map(|window| LimitItem {
            limit: PanelLimit {
                account_id: account.id.clone(),
                window: window.id.clone(),
            },
            provider: account.provider.clone(),
            title: format!(
                "{} — {}",
                account.provider_name,
                window_label(lang, &window.id, &window.label)
            ),
            subtitle: format!(
                "{who} · {}",
                percent_reading(lang, window.remaining_percent, ValueMode::Left)
            ),
        })
        .collect()
}

fn missing_limit(lang: Lang, limit: &PanelLimit) -> LimitItem {
    LimitItem {
        limit: limit.clone(),
        provider: String::new(),
        title: lang.tr("Pinned limit (not available now)").to_owned(),
        subtitle: format!("{} · {}", limit.account_id, limit.window),
    }
}

#[must_use]
pub fn limit_items(lang: Lang, state: &State, display: &Display) -> Vec<LimitItem> {
    let mut items: Vec<LimitItem> = display
        .panel_limits
        .iter()
        .map(|limit| missing_limit(lang, limit))
        .collect();
    for item in visible_accounts(state).flat_map(|account| account_limits(lang, display, account)) {
        match items.iter_mut().find(|known| known.limit == item.limit) {
            Some(known) => *known = item,
            None => items.push(item),
        }
    }
    items
}

#[must_use]
pub fn limits_summary(lang: Lang, chosen: usize) -> String {
    if chosen == 0 {
        return lang
            .tr("None chosen · the two most critical are shown")
            .to_owned();
    }
    fill(
        lang.tr("{count} of {max} chosen · shown in this order"),
        &[
            ("count", &chosen.to_string()),
            ("max", &MAX_PANEL_LIMITS.to_string()),
        ],
    )
}

#[must_use]
pub fn star_items(state: &State) -> Vec<StarItem> {
    visible_accounts(state)
        .map(|account| {
            let mut parts: Vec<&str> = vec![account.provider_name.as_str()];
            parts.extend(account.plan.as_deref());
            if account.label.is_some() {
                parts.extend(account.email.as_deref());
            }
            StarItem {
                account_id: account.id.clone(),
                provider: account.provider.clone(),
                title: account_name(account),
                subtitle: parts.join(" · "),
            }
        })
        .collect()
}

#[must_use]
pub fn star_label(lang: Lang, starred: bool) -> &'static str {
    if starred {
        lang.tr("Always open")
    } else {
        lang.tr("On demand")
    }
}

#[must_use]
pub fn account_providers(state: &State) -> Vec<ProviderItem> {
    let mut providers: Vec<ProviderItem> = Vec::new();
    for account in &state.accounts {
        if providers.iter().all(|known| known.id != account.provider) {
            providers.push(ProviderItem {
                id: account.provider.clone(),
                name: account.provider_name.clone(),
            });
        }
    }
    providers
}

#[must_use]
pub fn thresholds_summary(
    lang: Lang,
    notifications: &Notifications,
    providers: &[ProviderItem],
) -> String {
    let differing = providers
        .iter()
        .filter(|provider| notifications.provider_thresholds.contains_key(&provider.id))
        .count();
    if differing == 0 {
        return lang.tr("Every provider uses the default").to_owned();
    }
    let forms = [
        "{count} provider differs from the default",
        "{count} providers differ from the default",
    ];
    fill(
        lang.tr_plural(forms, u64::try_from(differing).unwrap_or(u64::MAX)),
        &[("count", &differing.to_string())],
    )
}

#[must_use]
pub fn almost_out_subtitle(lang: Lang, threshold: u8) -> String {
    fill(
        lang.tr("A limit drops under {percent}% left"),
        &[("percent", &threshold.to_string())],
    )
}

#[cfg(test)]
#[path = "picks_tests.rs"]
mod tests;
