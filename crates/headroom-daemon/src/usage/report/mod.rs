pub mod request;
pub mod rows;

use std::collections::BTreeSet;
use std::path::Path;

use headroom_core::usage::PriceBook;
use jiff::tz::TimeZone;
use rusqlite::Connection;

use crate::catalog::ProviderCatalog;
use crate::clock::Clock;
use crate::error::{CommandError, StorageError};
use crate::home::{HomeDisplay, UsageHome};
use crate::storage::{Storage, cursors};
use crate::usage::breakdown::{self, SpendQuery};
use request::SpendRequest;
use rows::SpendReport;

pub struct ReportContext<'a> {
    pub prices: &'a dyn PriceBook,
    pub tz: &'a TimeZone,
    pub homes: &'a HomeDisplay,
}

pub fn build(
    conn: &Connection,
    usage_homes: &BTreeSet<UsageHome>,
    request: &SpendRequest,
    ctx: &ReportContext<'_>,
) -> Result<SpendReport, StorageError> {
    let selected: BTreeSet<UsageHome> = usage_homes
        .iter()
        .filter(|home| {
            request
                .provider
                .as_ref()
                .is_none_or(|id| &home.provider == id)
        })
        .cloned()
        .collect();
    let query = SpendQuery {
        prices: ctx.prices,
        tz: ctx.tz,
        range: request.range,
        group_by: request.by,
    };
    let breakdown = breakdown::query(conn, &selected, &query)?;
    Ok(rows::report(breakdown, request, ctx.homes))
}

pub struct OnceSpendContext<'a> {
    pub price_book: &'a dyn PriceBook,
    pub clock: &'a dyn Clock,
    pub tz: &'a TimeZone,
    pub homes: &'a HomeDisplay,
    pub catalog: &'a ProviderCatalog,
}

/// Answers a `GetSpend` query from the database alone, for when no daemon is running.
pub fn spend_report_once(
    db_path: &Path,
    query: &str,
    ctx: &OnceSpendContext<'_>,
) -> Result<SpendReport, CommandError> {
    let request = request::resolve(query, ctx.tz, ctx.clock.now(), ctx.catalog)?;
    let storage = Storage::open(db_path)?;
    let report_ctx = ReportContext {
        prices: ctx.price_book,
        tz: ctx.tz,
        homes: ctx.homes,
    };
    Ok(storage.blocking(|conn| {
        let homes: BTreeSet<UsageHome> = cursors::homes(conn)?
            .into_iter()
            .filter(|home| home.home.is_dir())
            .collect();
        build(conn, &homes, &request, &report_ctx)
    })?)
}

#[cfg(test)]
mod tests;
