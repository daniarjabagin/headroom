mod account;
mod headline;
pub mod payload;
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
    let usage = model
        .usage
        .iter()
        .filter(|(home, _)| homes.contains(home))
        .map(|(home, summary)| usage::usage_view(home, summary, ctx))
        .collect();
    StatePayload {
        version: STATE_VERSION,
        generated_at: ctx.now,
        headline: headline::headline(&accounts, &model.settings.headline),
        accounts,
        usage,
    }
}

#[cfg(test)]
mod tests;
