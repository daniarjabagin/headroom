use std::collections::BTreeMap;

use headroom_core::account::ProviderId;
use headroom_core::usage::{PeriodUsage, UsageSummary, UsageTotals};

use super::AssembleContext;
use super::models::{ModelMerge, ProviderModel, merged_top_models, top_models};
use super::payload::{PeriodSpendView, ProviderSpendView, SpendView};
use super::projects::ProjectMerge;
use crate::home::UsageHome;

pub type HomeSummary<'a> = (&'a UsageHome, &'a UsageSummary);

type Pick = fn(&UsageSummary) -> &PeriodUsage;

#[must_use]
pub fn spend(homes: &[HomeSummary<'_>], ctx: &AssembleContext<'_>) -> SpendView {
    SpendView {
        today: period(homes, ctx, |s| &s.today),
        yesterday: period(homes, ctx, |s| &s.yesterday),
        last_7_days: period(homes, ctx, |s| &s.last_7_days),
        last_30_days: period(homes, ctx, |s| &s.last_30_days),
    }
}

#[derive(Default)]
struct ProviderMerge {
    totals: UsageTotals,
    models: ModelMerge,
}

fn period(homes: &[HomeSummary<'_>], ctx: &AssembleContext<'_>, pick: Pick) -> PeriodSpendView {
    let mut providers: BTreeMap<ProviderId, ProviderMerge> = BTreeMap::new();
    let mut projects = ProjectMerge::default();
    let mut totals = UsageTotals::default();
    for (home, summary) in homes {
        let usage = pick(summary);
        let merge = providers.entry(home.provider.clone()).or_default();
        merge.totals.absorb(&usage.totals);
        merge.models.add(&usage.models);
        projects.add(&home.provider, &usage.projects);
        totals.absorb(&usage.totals);
    }
    let (views, merged): (Vec<_>, Vec<_>) = providers
        .into_iter()
        .map(|(id, merge)| provider_spend(id, merge, ctx))
        .filter(|(view, _)| has_usage(view))
        .unzip();
    let by_provider = ranked(views);
    let (models, models_other) = merged_top_models(merged.into_iter().flatten().collect());
    let projects = projects.finish(&totals, ctx);
    PeriodSpendView {
        cost_usd_micros: by_provider
            .iter()
            .fold(0, |sum, p| sum.saturating_add(p.cost_usd_micros)),
        total_tokens: by_provider
            .iter()
            .fold(0, |sum, p| sum.saturating_add(p.total_tokens)),
        partial: by_provider.iter().any(|p| p.partial),
        cost_per_mtok_usd_micros: totals.cost_per_mtok().map(|cost| cost.0),
        by_provider,
        models,
        models_other,
        projects: projects.listed,
        projects_other: projects.other,
    }
}

fn provider_spend(
    provider: ProviderId,
    merge: ProviderMerge,
    ctx: &AssembleContext<'_>,
) -> (ProviderSpendView, Vec<ProviderModel>) {
    let provider_name = ctx.catalog.display_name(&provider).to_owned();
    let ranked = merge.models.ranked();
    let (models, models_other) = top_models(&ranked);
    let merged = ranked
        .into_iter()
        .map(|usage| ProviderModel {
            provider: provider.clone(),
            provider_name: provider_name.clone(),
            usage,
        })
        .collect();
    let totals = merge.totals;
    let view = ProviderSpendView {
        provider,
        provider_name,
        cost_usd_micros: totals.cost.0,
        total_tokens: totals.tokens.total().0,
        partial: totals.is_partial(),
        cost_per_mtok_usd_micros: totals.cost_per_mtok().map(|cost| cost.0),
        models,
        models_other,
    };
    (view, merged)
}

fn has_usage(spend: &ProviderSpendView) -> bool {
    spend.cost_usd_micros != 0 || spend.total_tokens != 0
}

fn ranked(mut providers: Vec<ProviderSpendView>) -> Vec<ProviderSpendView> {
    providers.sort_by(|a, b| {
        b.cost_usd_micros
            .cmp(&a.cost_usd_micros)
            .then_with(|| a.provider.as_str().cmp(b.provider.as_str()))
    });
    providers
}

#[cfg(test)]
#[path = "spend_tests.rs"]
mod tests;
