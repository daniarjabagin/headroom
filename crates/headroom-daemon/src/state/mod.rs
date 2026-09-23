mod account;
mod activity;
mod headline;
mod models;
pub mod payload;
mod spend;
pub mod status;
mod usage;

use jiff::Timestamp;
use jiff::tz::TimeZone;

use crate::catalog::ProviderCatalog;
use crate::home::HomeDisplay;
use crate::model::Model;
use payload::{STATE_VERSION, StatePayload};

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
    let full_usage: Vec<_> = listed
        .into_iter()
        .map(|(home, summary)| usage::usage_view(home, summary, ctx))
        .collect();
    let spend = spend::spend(&full_usage);
    let usage = full_usage.into_iter().map(usage::with_top_models).collect();
    StatePayload {
        version: STATE_VERSION,
        generated_at: ctx.now,
        next_refresh_at: activity::next_refresh_at(model),
        last_success_at: activity::last_success_at(model),
        offline: activity::offline(model),
        display: model.settings.display.clone(),
        headline: headline::headline(&accounts, &model.settings.headline),
        accounts,
        spend,
        usage,
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod lapse_tests;
