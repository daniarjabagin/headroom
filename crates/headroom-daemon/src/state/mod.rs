mod account;
mod account_collapse;
mod account_recovery;
mod account_usage;
mod activity;
mod combined;
mod combined_pace;
mod headline;
mod models;
mod panel_items;
pub mod payload;
mod projects;
mod provider_status;
mod refresh;
mod spend;
pub mod status;
mod update;
mod usage;

#[cfg(test)]
mod test_views;

use jiff::Timestamp;
use jiff::tz::TimeZone;

use crate::catalog::ProviderCatalog;
use crate::home::HomeDisplay;
use crate::model::Model;
use payload::{APP_VERSION, STATE_VERSION, StatePayload};

pub struct AssembleContext<'a> {
    pub now: Timestamp,
    pub tz: &'a TimeZone,
    pub homes: &'a HomeDisplay,
    pub catalog: &'a ProviderCatalog,
}

#[must_use]
pub fn assemble(model: &Model, ctx: &AssembleContext<'_>) -> StatePayload {
    let accounts: Vec<_> = model
        .active_accounts()
        .map(|record| account::account_view(record, model, ctx))
        .collect();
    let mut listed: Vec<_> = model
        .usage
        .iter()
        .filter(|(home, _)| model.usage_homes.contains(home))
        .collect();
    listed.sort_by_key(|(home, _)| (ctx.catalog.rank(&home.provider), &home.home));
    let display = &model.settings.display;
    let combined = account_collapse::with_group_collapse(
        combined::combined(&accounts, display.combine_accounts),
        display,
    );
    let spend = spend::spend(&listed, ctx);
    let usage = account_usage::shown_usage(model, &listed)
        .into_iter()
        .map(|(home, summary)| usage::usage_view(home, &summary, ctx))
        .collect();
    StatePayload {
        version: STATE_VERSION,
        app_version: Some(APP_VERSION.to_owned()),
        generated_at: ctx.now,
        next_refresh_at: activity::next_refresh_at(model),
        last_success_at: activity::last_success_at(model),
        offline: activity::offline(model),
        update: update::update_view(model),
        update_check: update::update_check_view(model),
        display: display.clone(),
        headline: headline::headline(&accounts, &combined, &model.settings.headline),
        panel_items: panel_items::panel_items(&accounts, &combined, &model.settings),
        panel_tone: panel_items::panel_tone(&accounts, &combined),
        accounts,
        combined,
        provider_status: provider_status::provider_status(model, ctx),
        spend,
        usage,
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod forecast_state_tests;

#[cfg(test)]
mod lapse_tests;

#[cfg(test)]
mod combined_state_tests;

#[cfg(test)]
mod panel_state_tests;

#[cfg(test)]
mod recovery_tests;

#[cfg(test)]
mod rate_limit_tests;

#[cfg(test)]
mod spend_state_tests;

#[cfg(test)]
mod usage_state_tests;
