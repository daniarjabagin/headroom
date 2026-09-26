use crate::i18n::Lang;
use crate::payload::{
    ModelUsage, OtherModels, PeriodSpend, ProviderModelUsage, ProviderSpend, SpendUnit,
};

use super::breakdown::{
    BarPart, BreakdownList, BreakdownRow, Folded, MODEL_FORMS, Scale, counted, model_row, other_row,
};
use super::spend_view::Basis;

type Listed<'a> = (&'a str, &'a ModelUsage);

struct Fold {
    total: Folded,
    bar: Vec<BarPart>,
}

fn daemon_listed(models: &[ProviderModelUsage]) -> Vec<Listed<'_>> {
    models
        .iter()
        .map(|model| (model.provider.as_str(), &model.usage))
        .collect()
}

fn daemon_fold(scale: Scale, other: &OtherModels) -> Fold {
    Fold {
        total: Folded::from(other),
        bar: vec![scale.part(None, other.cost_usd_micros, other.total_tokens)],
    }
}

fn provider_listed(providers: &[ProviderSpend]) -> Vec<Listed<'_>> {
    providers
        .iter()
        .flat_map(|spend| {
            spend
                .models
                .iter()
                .map(move |model| (spend.provider.as_str(), model))
        })
        .collect()
}

fn provider_fold(scale: Scale, providers: &[ProviderSpend]) -> Option<Fold> {
    let others: Vec<(&str, &OtherModels)> = providers
        .iter()
        .filter_map(|spend| Some((spend.provider.as_str(), spend.models_other.as_ref()?)))
        .collect();
    if others.is_empty() {
        return None;
    }
    let total = others.iter().fold(Folded::default(), |total, (_, other)| {
        total.summed(Folded::from(*other))
    });
    let bar = others
        .iter()
        .map(|(provider, other)| {
            scale.part(
                Some((*provider).to_owned()),
                other.cost_usd_micros,
                other.total_tokens,
            )
        })
        .collect();
    Some(Fold { total, bar })
}

fn sort_by_measure(scale: Scale, listed: &mut [Listed<'_>]) {
    listed.sort_by(|a, b| {
        let key = |model: &ModelUsage| scale.of(model.cost_usd_micros, model.total_tokens);
        key(b.1)
            .cmp(&key(a.1))
            .then(b.1.total_tokens.cmp(&a.1.total_tokens))
            .then(a.1.model.cmp(&b.1.model))
    });
}

fn assemble(scale: Scale, listed: &[Listed<'_>], fold: Option<Fold>) -> BreakdownList {
    let mut rows: Vec<BreakdownRow> = listed
        .iter()
        .map(|(provider, model)| model_row(scale, provider, model))
        .collect();
    let folded = fold.as_ref().map_or(0, |fold| fold.total.count);
    if let Some(fold) = fold {
        let detail = counted(scale.lang, MODEL_FORMS, fold.total.count);
        rows.push(other_row(scale, detail, fold.total, fold.bar));
    }
    let total = u64::try_from(listed.len())
        .unwrap_or(u64::MAX)
        .saturating_add(folded);
    BreakdownList {
        caption: counted(scale.lang, MODEL_FORMS, total),
        rows,
    }
}

#[must_use]
pub fn model_list(
    lang: Lang,
    unit: SpendUnit,
    basis: Basis,
    period: &PeriodSpend,
) -> BreakdownList {
    let scale = Scale::new(lang, unit, basis, period);
    let (mut listed, fold) = match &period.models {
        Some(models) => (
            daemon_listed(models),
            period
                .models_other
                .as_ref()
                .map(|other| daemon_fold(scale, other)),
        ),
        None => (
            provider_listed(&period.by_provider),
            provider_fold(scale, &period.by_provider),
        ),
    };
    if period.models.is_none() || scale.basis == Basis::Tokens {
        sort_by_measure(scale, &mut listed);
    }
    assemble(scale, &listed, fold)
}

#[cfg(test)]
#[path = "models_tests.rs"]
mod tests;
