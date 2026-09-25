use std::collections::BTreeMap;
use std::path::Path;

use headroom_core::account::ProviderId;
use headroom_core::usage::UsageTotals;
use jiff::ToSpan;
use jiff::civil::Date;
use serde::{Deserialize, Serialize};

use super::request::SpendRequest;
use crate::home::HomeDisplay;
use crate::state::payload::TokensView;
use crate::usage::breakdown::{Breakdown, GroupBy, GroupKey, SpendRow};
use crate::usage::share::share_permille;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpendReport {
    pub since: Date,
    pub until: Date,
    pub by: GroupBy,
    pub rows: Vec<ReportRow>,
    pub total: ReportRow,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReportRow {
    pub key: Option<String>,
    pub provider: Option<ProviderId>,
    pub tokens: TokensView,
    pub cost_usd_micros: i64,
    pub partial: bool,
    pub unpriced_tokens: u64,
    pub cost_per_mtok_usd_micros: Option<i64>,
    pub share_permille: u32,
}

#[must_use]
pub fn report(breakdown: Breakdown, request: &SpendRequest, homes: &HomeDisplay) -> SpendReport {
    let whole = breakdown.totals;
    let rows = if request.by == GroupBy::Day {
        every_day(breakdown.rows, request)
    } else {
        breakdown.rows
    };
    SpendReport {
        since: request.since,
        until: request.until,
        by: request.by,
        rows: rows
            .iter()
            .map(|row| {
                let (key, provider) = labels(&row.key, homes);
                report_row(key, provider, &row.totals, &whole)
            })
            .collect(),
        total: report_row(None, None, &whole, &whole),
    }
}

fn every_day(rows: Vec<SpendRow>, request: &SpendRequest) -> Vec<SpendRow> {
    let mut by_day: BTreeMap<GroupKey, UsageTotals> =
        rows.into_iter().map(|row| (row.key, row.totals)).collect();
    request
        .since
        .series(1.day())
        .take_while(|day| *day <= request.until)
        .map(|day| {
            let key = GroupKey::Day(day);
            let totals = by_day.remove(&key).unwrap_or_default();
            SpendRow { key, totals }
        })
        .collect()
}

fn labels(key: &GroupKey, homes: &HomeDisplay) -> (Option<String>, Option<ProviderId>) {
    match key {
        GroupKey::Model(provider, model) => (Some(model.clone()), Some(provider.clone())),
        GroupKey::Project(project) => (
            project.as_deref().map(|path| homes.show(Path::new(path))),
            None,
        ),
        GroupKey::Provider(provider) => (Some(provider.to_string()), Some(provider.clone())),
        GroupKey::Day(day) => (Some(day.to_string()), None),
    }
}

fn report_row(
    key: Option<String>,
    provider: Option<ProviderId>,
    totals: &UsageTotals,
    whole: &UsageTotals,
) -> ReportRow {
    ReportRow {
        key,
        provider,
        tokens: TokensView::of(&totals.tokens),
        cost_usd_micros: totals.cost.0,
        partial: totals.is_partial(),
        unpriced_tokens: totals.unpriced_tokens.0,
        cost_per_mtok_usd_micros: totals.cost_per_mtok().map(|cost| cost.0),
        share_permille: share_permille(totals, whole),
    }
}
