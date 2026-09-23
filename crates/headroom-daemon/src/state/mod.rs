mod account;
mod activity;
mod headline;
pub mod payload;
mod spend;
pub mod status;
mod usage;

use jiff::Timestamp;
use jiff::tz::TimeZone;

use crate::home::HomeDisplay;
use crate::model::Model;
use payload::{STATE_VERSION, StatePayload};

pub struct AssembleContext<'a> {
    pub now: Timestamp,
    pub tz: &'a TimeZone,
    pub homes: &'a HomeDisplay,
}

#[must_use]
pub fn assemble(model: &Model, ctx: &AssembleContext<'_>) -> StatePayload {
    let accounts: Vec<_> = model
        .active_accounts()
        .map(|record| account::account_view(record, model, ctx))
        .collect();
    let homes = model.usage_homes();
    let usage: Vec<_> = model
        .usage
        .iter()
        .filter(|(home, _)| homes.contains(home))
        .map(|(home, summary)| usage::usage_view(home, summary, ctx))
        .collect();
    StatePayload {
        version: STATE_VERSION,
        generated_at: ctx.now,
        next_refresh_at: activity::next_refresh_at(model),
        last_success_at: activity::last_success_at(model),
        offline: activity::offline(model),
        headline: headline::headline(&accounts, &model.settings.headline),
        accounts,
        spend: spend::spend(&usage),
        usage,
    }
}

#[cfg(test)]
mod tests;
