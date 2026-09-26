use std::collections::BTreeMap;

use std::cmp::Ordering;

use headroom_core::account::ProviderId;
use headroom_core::usage::{ModelUsage, UsageTotals, most_expensive_first};

use super::payload::{ModelView, OtherModelsView, ProviderModelView};

pub const TOP_MODELS: usize = 5;

#[must_use]
pub fn model_view(usage: &ModelUsage) -> ModelView {
    ModelView {
        model: usage.model.clone(),
        total_tokens: usage.totals.tokens.total().0,
        cost_usd_micros: usage.totals.cost.0,
        partial: usage.totals.is_partial(),
        cost_per_mtok_usd_micros: usage.totals.cost_per_mtok().map(|cost| cost.0),
    }
}

#[derive(Debug, Default)]
pub struct ModelMerge {
    by_name: BTreeMap<String, UsageTotals>,
}

impl ModelMerge {
    pub fn add(&mut self, models: &[ModelUsage]) {
        for usage in models {
            self.by_name
                .entry(usage.model.clone())
                .or_default()
                .absorb(&usage.totals);
        }
    }

    #[must_use]
    pub fn ranked(self) -> Vec<ModelUsage> {
        let mut models: Vec<ModelUsage> = self
            .by_name
            .into_iter()
            .map(|(model, totals)| ModelUsage { model, totals })
            .collect();
        models.sort_by(|a, b| {
            most_expensive_first(&a.totals, &b.totals).then_with(|| a.model.cmp(&b.model))
        });
        models
    }
}

#[derive(Debug, Clone)]
pub struct ProviderModel {
    pub provider: ProviderId,
    pub provider_name: String,
    pub usage: ModelUsage,
}

#[must_use]
pub fn top_models(ranked: &[ModelUsage]) -> (Vec<ModelView>, Option<OtherModelsView>) {
    let (listed, rest) = split_top(ranked);
    (
        listed.iter().map(model_view).collect(),
        other_models(rest.iter().map(|usage| &usage.totals)),
    )
}

#[must_use]
pub fn merged_top_models(
    mut models: Vec<ProviderModel>,
) -> (Vec<ProviderModelView>, Option<OtherModelsView>) {
    models.sort_by(merged_order);
    let (listed, rest) = split_top(&models);
    (
        listed.iter().map(provider_model_view).collect(),
        other_models(rest.iter().map(|entry| &entry.usage.totals)),
    )
}

fn merged_order(a: &ProviderModel, b: &ProviderModel) -> Ordering {
    most_expensive_first(&a.usage.totals, &b.usage.totals)
        .then_with(|| a.usage.model.cmp(&b.usage.model))
        .then_with(|| a.provider.as_str().cmp(b.provider.as_str()))
}

fn provider_model_view(entry: &ProviderModel) -> ProviderModelView {
    ProviderModelView {
        provider: entry.provider.clone(),
        provider_name: entry.provider_name.clone(),
        model: model_view(&entry.usage),
    }
}

fn split_top<T>(ranked: &[T]) -> (&[T], &[T]) {
    if ranked.len() <= TOP_MODELS + 1 {
        return (ranked, &[]);
    }
    ranked.split_at(TOP_MODELS)
}

fn other_models<'a>(rest: impl Iterator<Item = &'a UsageTotals>) -> Option<OtherModelsView> {
    let mut count = 0;
    let mut totals = UsageTotals::default();
    for usage in rest {
        count += 1;
        totals.absorb(usage);
    }
    (count > 0).then(|| OtherModelsView {
        count,
        total_tokens: totals.tokens.total().0,
        cost_usd_micros: totals.cost.0,
        partial: totals.is_partial(),
        cost_per_mtok_usd_micros: totals.cost_per_mtok().map(|cost| cost.0),
    })
}

#[cfg(test)]
#[path = "models_tests.rs"]
mod tests;
