mod account;
mod activity;
mod combined;
mod combined_pace;
mod headline;
mod models;
pub mod payload;
mod projects;
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
    let combined = combined::combined(&accounts, model.settings.display.combine_accounts);
    let spend = spend::spend(&listed, ctx);
    let usage = listed
        .into_iter()
        .map(|(home, summary)| usage::with_top_models(usage::usage_view(home, summary, ctx)))
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
        display: model.settings.display.clone(),
        headline: headline::headline(&accounts, &combined, &model.settings.headline),
        accounts,
        combined,
        spend,
        usage,
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod lapse_tests;

#[cfg(test)]
mod combined_state_tests;

#[cfg(test)]
mod spend_state_tests;
