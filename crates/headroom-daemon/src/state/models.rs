use std::collections::BTreeMap;

use headroom_core::usage::{ModelUsage, UsageTotals, most_expensive_first};

use super::payload::{ModelView, OtherModelsView};

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

#[must_use]
pub fn top_models(ranked: &[ModelUsage]) -> (Vec<ModelView>, Option<OtherModelsView>) {
    if ranked.len() <= TOP_MODELS + 1 {
        return (ranked.iter().map(model_view).collect(), None);
    }
    let (listed, rest) = ranked.split_at(TOP_MODELS);
    (
        listed.iter().map(model_view).collect(),
        Some(other_models(rest)),
    )
}

fn other_models(rest: &[ModelUsage]) -> OtherModelsView {
    let totals = rest.iter().fold(UsageTotals::default(), |mut sum, usage| {
        sum.absorb(&usage.totals);
        sum
    });
    OtherModelsView {
        count: rest.len(),
        total_tokens: totals.tokens.total().0,
        cost_usd_micros: totals.cost.0,
        partial: totals.is_partial(),
        cost_per_mtok_usd_micros: totals.cost_per_mtok().map(|cost| cost.0),
    }
}

#[cfg(test)]
#[path = "models_tests.rs"]
mod tests;
