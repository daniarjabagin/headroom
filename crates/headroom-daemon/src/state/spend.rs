use std::collections::BTreeMap;

use headroom_core::account::ProviderId;
use headroom_core::usage::{PeriodUsage, UsageSummary, UsageTotals};

use super::AssembleContext;
use super::models::{ModelMerge, top_models};
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
    let by_provider = ranked(
        providers
            .into_iter()
            .map(|(id, merge)| provider_view(id, merge, ctx))
            .filter(has_usage)
            .collect(),
    );
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
        projects: projects.listed,
        projects_other: projects.other,
    }
}

fn provider_view(
    provider: ProviderId,
    merge: ProviderMerge,
    ctx: &AssembleContext<'_>,
) -> ProviderSpendView {
    let (models, models_other) = top_models(&merge.models.ranked());
    let totals = merge.totals;
    ProviderSpendView {
        provider_name: ctx.catalog.display_name(&provider).to_owned(),
        provider,
        cost_usd_micros: totals.cost.0,
        total_tokens: totals.tokens.total().0,
        partial: totals.is_partial(),
        cost_per_mtok_usd_micros: totals.cost_per_mtok().map(|cost| cost.0),
        models,
        models_other,
    }
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
