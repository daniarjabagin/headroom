use std::collections::BTreeMap;
use std::path::Path;

use headroom_core::account::ProviderId;
use headroom_core::usage::{ProjectUsage, TOP_PROJECTS, UsageTotals, top_projects};

use super::AssembleContext;
use super::payload::{OtherProjectsView, ProjectProviderView, ProjectSpendView};
use crate::usage::share::share_permille;

pub const MIN_LISTED_PERMILLE: u32 = 50;

#[derive(Debug, Default)]
pub struct ProjectMerge {
    by_project: BTreeMap<Option<String>, ProjectSlot>,
}

#[derive(Debug, Default)]
struct ProjectSlot {
    totals: UsageTotals,
    providers: BTreeMap<ProviderId, UsageTotals>,
}

pub struct ProjectsView {
    pub listed: Vec<ProjectSpendView>,
    pub other: Option<OtherProjectsView>,
}

impl ProjectMerge {
    pub fn add(&mut self, provider: &ProviderId, projects: &[ProjectUsage]) {
        for usage in projects {
            let slot = self.by_project.entry(usage.project.clone()).or_default();
            slot.totals.absorb(&usage.totals);
            slot.providers
                .entry(provider.clone())
                .or_default()
                .absorb(&usage.totals);
        }
    }

    #[must_use]
    pub fn finish(self, period: &UsageTotals, ctx: &AssembleContext<'_>) -> ProjectsView {
        let all: Vec<ProjectUsage> = self
            .by_project
            .iter()
            .map(|(project, slot)| ProjectUsage {
                project: project.clone(),
                totals: slot.totals.clone(),
            })
            .collect();
        let count = all.len();
        let sorted = top_projects(all, count).top;
        let listed = listed_count(&sorted, period);
        let cut = top_projects(sorted, listed);
        ProjectsView {
            listed: cut
                .top
                .iter()
                .map(|usage| self.project_view(usage, period, ctx))
                .collect(),
            other: cut
                .other
                .map(|totals| other_view(&totals, count - listed, period)),
        }
    }

    fn project_view(
        &self,
        usage: &ProjectUsage,
        period: &UsageTotals,
        ctx: &AssembleContext<'_>,
    ) -> ProjectSpendView {
        let totals = &usage.totals;
        ProjectSpendView {
            project: usage
                .project
                .as_deref()
                .map(|path| ctx.homes.show(Path::new(path))),
            cost_usd_micros: totals.cost.0,
            total_tokens: totals.tokens.total().0,
            partial: totals.is_partial(),
            share_permille: share_permille(totals, period),
            by_provider: self
                .by_project
                .get(&usage.project)
                .map(|slot| providers_view(slot, ctx))
                .unwrap_or_default(),
        }
    }
}

fn listed_count(sorted: &[ProjectUsage], period: &UsageTotals) -> usize {
    let listed = sorted
        .iter()
        .take(TOP_PROJECTS)
        .take_while(|usage| share_permille(&usage.totals, period) >= MIN_LISTED_PERMILLE)
        .count();
    if sorted.len() - listed == 1 {
        sorted.len()
    } else {
        listed
    }
}

fn providers_view(slot: &ProjectSlot, ctx: &AssembleContext<'_>) -> Vec<ProjectProviderView> {
    let mut providers: Vec<ProjectProviderView> = slot
        .providers
        .iter()
        .map(|(provider, totals)| ProjectProviderView {
            provider: provider.clone(),
            provider_name: ctx.catalog.display_name(provider).to_owned(),
            cost_usd_micros: totals.cost.0,
            total_tokens: totals.tokens.total().0,
        })
        .collect();
    providers.sort_by(|a, b| {
        b.cost_usd_micros
            .cmp(&a.cost_usd_micros)
            .then_with(|| a.provider.as_str().cmp(b.provider.as_str()))
    });
    providers
}

fn other_view(totals: &UsageTotals, count: usize, period: &UsageTotals) -> OtherProjectsView {
    OtherProjectsView {
        count,
        cost_usd_micros: totals.cost.0,
        total_tokens: totals.tokens.total().0,
        partial: totals.is_partial(),
        share_permille: share_permille(totals, period),
    }
}
