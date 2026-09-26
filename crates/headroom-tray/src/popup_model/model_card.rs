use crate::i18n::Lang;
use crate::numbers::compact_tokens_text;
use crate::payload::{OtherModels, ProviderSpend, SpendUnit};

use super::breakdown::{Folded, MODEL_FORMS, Scale, counted, model_figures};
use super::spend_view::{Basis, Figures, permille, proportion, whole_share_text};

#[derive(Debug, Clone, PartialEq)]
pub struct ModelLine {
    pub name: String,
    pub detail: Option<String>,
    pub value: String,
    pub share: String,
    pub tokens: String,
    pub fraction: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ModelCard {
    pub title: String,
    pub total: String,
    pub lines: Vec<ModelLine>,
    pub folded: bool,
}

fn model_line(scale: Scale, name: String, detail: Option<String>, figures: Figures) -> ModelLine {
    let part = scale.of(figures.cost_micros, figures.tokens);
    ModelLine {
        name,
        detail,
        value: scale.value(figures),
        share: whole_share_text(permille(part, scale.whole)),
        tokens: compact_tokens_text(scale.lang, figures.tokens),
        fraction: proportion(part, scale.whole),
    }
}

fn other_line(scale: Scale, other: &OtherModels) -> ModelLine {
    model_line(
        scale,
        scale.lang.tr("Other").to_owned(),
        Some(counted(scale.lang, MODEL_FORMS, other.count)),
        Folded::from(other).figures(),
    )
}

#[must_use]
pub fn model_card(
    lang: Lang,
    unit: SpendUnit,
    basis: Basis,
    heading: &str,
    spend: &ProviderSpend,
) -> Option<ModelCard> {
    if spend.models.is_empty() && spend.models_other.is_none() {
        return None;
    }
    let scale = Scale::provider(lang, unit, basis, spend);
    let lines = spend
        .models
        .iter()
        .map(|model| model_line(scale, model.model.clone(), None, model_figures(model)))
        .chain(
            spend
                .models_other
                .iter()
                .map(|other| other_line(scale, other)),
        )
        .collect();
    Some(ModelCard {
        title: format!("{} · {heading}", spend.provider_name),
        total: scale.value(Figures::from(spend)),
        lines,
        folded: spend.models_other.is_some(),
    })
}
