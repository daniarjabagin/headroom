use std::collections::BTreeMap;

use headroom_core::usage::{ModelUsage, UsageTotals};

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
    pub fn ranked(self) -> Vec<ModelView> {
        let mut models: Vec<ModelView> = self
            .by_name
            .into_iter()
            .map(|(model, totals)| model_view(&ModelUsage { model, totals }))
            .collect();
        models.sort_by(|a, b| {
            b.cost_usd_micros
                .cmp(&a.cost_usd_micros)
                .then_with(|| b.total_tokens.cmp(&a.total_tokens))
                .then_with(|| a.model.cmp(&b.model))
        });
        models
    }
}

#[must_use]
pub fn top_models(mut ranked: Vec<ModelView>) -> (Vec<ModelView>, Option<OtherModelsView>) {
    if ranked.len() <= TOP_MODELS {
        return (ranked, None);
    }
    let rest = ranked.split_off(TOP_MODELS);
    (ranked, Some(other_models(&rest)))
}

fn other_models(rest: &[ModelView]) -> OtherModelsView {
    let start = OtherModelsView {
        count: rest.len(),
        ..OtherModelsView::default()
    };
    rest.iter().fold(start, |mut other, row| {
        other.total_tokens = other.total_tokens.saturating_add(row.total_tokens);
        other.cost_usd_micros = other.cost_usd_micros.saturating_add(row.cost_usd_micros);
        other.partial |= row.partial;
        other
    })
}

#[cfg(test)]
#[path = "models_tests.rs"]
mod tests;
