use crate::i18n::Lang;
use crate::payload::{Account, State, Status};
use crate::preferences::choices::account_name;
use crate::preferences::model::Settings;
use crate::preferences::registry::{AddMethod, ProviderInfo};

const MAX_NOT_INSTALLED: usize = 3;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Found {
    SignedIn { account_id: String, hidden: bool },
    SignedOut,
    NotInstalled,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FoundRow {
    pub provider: String,
    pub title: String,
    pub subtitle: String,
    pub found: Found,
}

#[must_use]
pub fn should_open(state: Option<&State>, settings: Option<&Settings>, dismissed: bool) -> bool {
    !dismissed
        && state.is_some_and(State::speaks_0_6)
        && settings.is_some_and(|settings| !settings.onboarding.completed)
}

fn signed_in_subtitle(lang: Lang, account: &Account) -> String {
    let mut parts: Vec<&str> = vec![lang.tr("Signed in")];
    parts.extend(account.plan.as_deref());
    parts.extend(account.email.as_deref());
    parts.join(" · ")
}

fn account_row(lang: Lang, account: &Account, shows_name: bool) -> FoundRow {
    let name = account_name(account);
    let title = if shows_name && name != account.provider_name {
        format!("{} · {name}", account.provider_name)
    } else {
        account.provider_name.clone()
    };
    if account.status == Status::SignedOut {
        return FoundRow {
            provider: account.provider.clone(),
            title,
            subtitle: lang.tr("Found, not signed in").to_owned(),
            found: Found::SignedOut,
        };
    }
    FoundRow {
        provider: account.provider.clone(),
        title,
        subtitle: signed_in_subtitle(lang, account),
        found: Found::SignedIn {
            account_id: account.id.clone(),
            hidden: account.hidden,
        },
    }
}

fn provider_row(lang: Lang, provider: &ProviderInfo, found: Found) -> FoundRow {
    let subtitle = match found {
        Found::NotInstalled => lang.tr("Not installed"),
        _ => lang.tr("Found, not signed in"),
    };
    FoundRow {
        provider: provider.id.clone(),
        title: provider.display_name.clone(),
        subtitle: subtitle.to_owned(),
        found,
    }
}

fn has_account(state: &State, provider: &str) -> bool {
    state
        .accounts
        .iter()
        .any(|account| account.provider == provider)
}

fn has_usage(state: &State, provider: &str) -> bool {
    state.usage.iter().any(|usage| usage.provider == provider)
}

fn signs_in_locally(provider: &ProviderInfo) -> bool {
    provider
        .methods
        .iter()
        .any(|method| matches!(method, AddMethod::CliLogin { .. }))
}

fn account_rows(lang: Lang, state: &State) -> Vec<FoundRow> {
    state
        .accounts
        .iter()
        .map(|account| {
            let twins = state
                .accounts
                .iter()
                .filter(|other| other.provider == account.provider)
                .count();
            account_row(lang, account, twins > 1)
        })
        .collect()
}

#[must_use]
pub fn found_rows(lang: Lang, state: &State, providers: &[ProviderInfo]) -> Vec<FoundRow> {
    let mut rows = account_rows(lang, state);
    let unaccounted = providers
        .iter()
        .filter(|provider| !has_account(state, &provider.id));
    let (found, missing): (Vec<&ProviderInfo>, Vec<&ProviderInfo>) =
        unaccounted.partition(|provider| has_usage(state, &provider.id));
    rows.extend(
        found
            .into_iter()
            .map(|provider| provider_row(lang, provider, Found::SignedOut)),
    );
    rows.extend(
        missing
            .into_iter()
            .filter(|provider| signs_in_locally(provider))
            .take(MAX_NOT_INSTALLED)
            .map(|provider| provider_row(lang, provider, Found::NotInstalled)),
    );
    rows
}

#[cfg(test)]
#[path = "found_tests.rs"]
mod tests;
