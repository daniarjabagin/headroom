use std::collections::BTreeMap;

use headroom_core::usage::ModelUsage;

use super::payload::ModelView;

#[must_use]
pub fn model_view(usage: &ModelUsage) -> ModelView {
    ModelView {
        model: usage.model.clone(),
        total_tokens: usage.totals.tokens.total().0,
        cost_usd_micros: usage.totals.cost.0,
        partial: usage.totals.is_partial(),
    }
}

#[derive(Debug, Default)]
pub struct ModelMerge {
    by_name: BTreeMap<String, ModelView>,
}

impl ModelMerge {
    pub fn add(&mut self, rows: &[ModelView]) {
        for row in rows {
            let slot = self
                .by_name
                .entry(row.model.clone())
                .or_insert_with(|| empty(&row.model));
            slot.total_tokens = slot.total_tokens.saturating_add(row.total_tokens);
            slot.cost_usd_micros = slot.cost_usd_micros.saturating_add(row.cost_usd_micros);
            slot.partial |= row.partial;
        }
    }

    #[must_use]
    pub fn ranked(self) -> Vec<ModelView> {
        let mut models: Vec<ModelView> = self.by_name.into_values().collect();
        models.sort_by(|a, b| {
            b.cost_usd_micros
                .cmp(&a.cost_usd_micros)
                .then_with(|| b.total_tokens.cmp(&a.total_tokens))
                .then_with(|| a.model.cmp(&b.model))
        });
        models
    }
}

fn empty(model: &str) -> ModelView {
    ModelView {
        model: model.to_owned(),
        total_tokens: 0,
        cost_usd_micros: 0,
        partial: false,
    }
}
