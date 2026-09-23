use std::collections::BTreeMap;

use headroom_core::account::ProviderKind;

use super::models::ModelMerge;
use super::payload::{PeriodSpendView, ProviderSpendView, SpendView, TotalsView, UsageView};

#[must_use]
pub fn spend(usage: &[UsageView]) -> SpendView {
    SpendView {
        today: period(usage, |u| &u.today),
        yesterday: period(usage, |u| &u.yesterday),
        last_30_days: period(usage, |u| &u.last_30_days),
    }
}

fn period(usage: &[UsageView], totals: impl Fn(&UsageView) -> &TotalsView) -> PeriodSpendView {
    let mut by_kind: BTreeMap<ProviderKind, (ProviderSpendView, ModelMerge)> = BTreeMap::new();
    for entry in usage {
        let (slot, models) = by_kind
            .entry(entry.provider)
            .or_insert_with(|| (empty(entry.provider), ModelMerge::default()));
        add(slot, totals(entry));
        models.add(&totals(entry).models);
    }
    let providers = by_kind.into_values().map(|(mut spend, models)| {
        spend.models = models.ranked();
        spend
    });
    let by_provider = ranked(providers.filter(has_usage).collect());
    PeriodSpendView {
        cost_usd_micros: by_provider
            .iter()
            .fold(0, |sum, p| sum.saturating_add(p.cost_usd_micros)),
        total_tokens: by_provider
            .iter()
            .fold(0, |sum, p| sum.saturating_add(p.total_tokens)),
        partial: by_provider.iter().any(|p| p.partial),
        by_provider,
    }
}

fn empty(provider: ProviderKind) -> ProviderSpendView {
    ProviderSpendView {
        provider,
        cost_usd_micros: 0,
        total_tokens: 0,
        partial: false,
        models: Vec::new(),
    }
}

fn add(slot: &mut ProviderSpendView, totals: &TotalsView) {
    slot.cost_usd_micros = slot.cost_usd_micros.saturating_add(totals.cost_usd_micros);
    slot.total_tokens = slot.total_tokens.saturating_add(totals.tokens.total);
    slot.partial |= totals.partial;
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
