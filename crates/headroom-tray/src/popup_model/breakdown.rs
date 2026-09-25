use crate::format::{middle_ellipsis, project_label};
use crate::i18n::{Lang, fill};
use crate::payload::{ModelUsage, PeriodSpend, ProjectSpend, ProviderSpend, SpendUnit};

use super::spend_view::{
    Basis, Figures, basis_of, measure, per_mtok_of, permille, proportion, share_text, unit_value,
};

const MODEL_ROWS: usize = 5;
const PROJECT_NAME_CHARS: usize = 24;
pub(super) const MODEL_FORMS: [&str; 2] = ["{count} model", "{count} models"];
const PROJECT_FORMS: [&str; 2] = ["{count} project", "{count} projects"];

#[derive(Debug, Clone, PartialEq)]
pub enum RowMark {
    Dot(String),
    Folder,
    None,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BarPart {
    pub provider: Option<String>,
    pub fraction: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BreakdownRow {
    pub mark: RowMark,
    pub name: String,
    pub detail: Option<String>,
    pub share: String,
    pub value: String,
    pub bar: Vec<BarPart>,
    pub tooltip: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BreakdownList {
    pub caption: String,
    pub rows: Vec<BreakdownRow>,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct Scale {
    pub(super) basis: Basis,
    pub(super) whole: u128,
    unit: SpendUnit,
    pub(super) lang: Lang,
}

impl Scale {
    pub(super) fn provider(
        lang: Lang,
        unit: SpendUnit,
        basis: Basis,
        spend: &ProviderSpend,
    ) -> Self {
        let basis = if basis == Basis::Cost && spend.cost_usd_micros <= 0 {
            Basis::Tokens
        } else {
            basis
        };
        Self {
            basis,
            whole: measure(basis, spend.cost_usd_micros, spend.total_tokens),
            unit,
            lang,
        }
    }

    fn new(lang: Lang, unit: SpendUnit, basis: Basis, period: &PeriodSpend) -> Self {
        let basis = basis_of(basis, period);
        Self {
            basis,
            whole: measure(basis, period.cost_usd_micros, period.total_tokens),
            unit,
            lang,
        }
    }

    pub(super) fn of(self, cost: i64, tokens: u64) -> u128 {
        measure(self.basis, cost, tokens)
    }

    fn share(self, cost: i64, tokens: u64) -> String {
        share_text(self.lang, permille(self.of(cost, tokens), self.whole))
    }

    fn part(self, provider: Option<String>, cost: i64, tokens: u64) -> BarPart {
        BarPart {
            provider,
            fraction: proportion(self.of(cost, tokens), self.whole),
        }
    }

    pub(super) fn value(self, figures: Figures) -> String {
        unit_value(self.lang, self.unit, figures)
    }

    fn tooltip(self, figures: Figures) -> String {
        format!(
            "{} · {}",
            crate::numbers::exact_usd(figures.cost_micros),
            crate::numbers::exact_tokens_text(self.lang, figures.tokens)
        )
    }
}

pub(super) fn counted(lang: Lang, forms: [&'static str; 2], count: u64) -> String {
    fill(
        lang.tr_plural(forms, count),
        &[("count", &count.to_string())],
    )
}

pub(super) fn model_figures(model: &ModelUsage) -> Figures {
    Figures {
        cost_micros: model.cost_usd_micros,
        tokens: model.total_tokens,
        per_mtok: model.cost_per_mtok_usd_micros,
    }
}

fn model_row(scale: Scale, provider: &str, model: &ModelUsage) -> BreakdownRow {
    let figures = model_figures(model);
    BreakdownRow {
        mark: RowMark::Dot(provider.to_owned()),
        name: model.model.clone(),
        detail: None,
        share: scale.share(model.cost_usd_micros, model.total_tokens),
        value: scale.value(figures),
        bar: vec![scale.part(
            Some(provider.to_owned()),
            model.cost_usd_micros,
            model.total_tokens,
        )],
        tooltip: scale.tooltip(figures),
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub(super) struct Folded {
    count: u64,
    cost: i64,
    tokens: u64,
    partial: bool,
}

impl Folded {
    pub(super) fn add(&mut self, count: u64, cost: i64, tokens: u64, partial: bool) {
        self.count = self.count.saturating_add(count);
        self.cost = self.cost.saturating_add(cost);
        self.tokens = self.tokens.saturating_add(tokens);
        self.partial |= partial;
    }

    pub(super) fn figures(self) -> Figures {
        Figures {
            cost_micros: self.cost,
            tokens: self.tokens,
            per_mtok: per_mtok_of(self.cost, self.tokens, self.partial),
        }
    }
}

fn provider_folds(
    ranked: &[(&str, &ModelUsage)],
    providers: &[ProviderSpend],
) -> Vec<(String, Folded)> {
    providers
        .iter()
        .filter_map(|spend| {
            let mut folded = Folded::default();
            for (_, model) in ranked.iter().filter(|(id, _)| *id == spend.provider) {
                folded.add(1, model.cost_usd_micros, model.total_tokens, model.partial);
            }
            if let Some(other) = &spend.models_other {
                folded.add(
                    other.count,
                    other.cost_usd_micros,
                    other.total_tokens,
                    other.partial,
                );
            }
            (folded.count > 0).then(|| (spend.provider.clone(), folded))
        })
        .collect()
}

fn other_row(scale: Scale, detail: String, folds: &[(Option<String>, Folded)]) -> BreakdownRow {
    let mut total = Folded::default();
    for (_, folded) in folds {
        total.add(folded.count, folded.cost, folded.tokens, folded.partial);
    }
    BreakdownRow {
        mark: RowMark::None,
        name: scale.lang.tr("Other").to_owned(),
        detail: Some(detail),
        share: scale.share(total.cost, total.tokens),
        value: scale.value(total.figures()),
        bar: folds
            .iter()
            .map(|(provider, folded)| scale.part(provider.clone(), folded.cost, folded.tokens))
            .collect(),
        tooltip: scale.tooltip(total.figures()),
    }
}

fn ranked_models(scale: Scale, period: &PeriodSpend) -> Vec<(&str, &ModelUsage)> {
    let mut models: Vec<(&str, &ModelUsage)> = period
        .by_provider
        .iter()
        .flat_map(|spend| {
            spend
                .models
                .iter()
                .map(move |model| (spend.provider.as_str(), model))
        })
        .collect();
    models.sort_by(|a, b| {
        let key = |model: &ModelUsage| scale.of(model.cost_usd_micros, model.total_tokens);
        key(b.1)
            .cmp(&key(a.1))
            .then(b.1.total_tokens.cmp(&a.1.total_tokens))
            .then(a.1.model.cmp(&b.1.model))
    });
    models
}

#[must_use]
pub fn model_list(
    lang: Lang,
    unit: SpendUnit,
    basis: Basis,
    period: &PeriodSpend,
) -> BreakdownList {
    let scale = Scale::new(lang, unit, basis, period);
    let ranked = ranked_models(scale, period);
    let shown = ranked.len().min(MODEL_ROWS);
    let mut rows: Vec<BreakdownRow> = ranked[..shown]
        .iter()
        .map(|(provider, model)| model_row(scale, provider, model))
        .collect();
    let folds: Vec<(Option<String>, Folded)> =
        provider_folds(&ranked[shown..], &period.by_provider)
            .into_iter()
            .map(|(provider, folded)| (Some(provider), folded))
            .collect();
    let folded: u64 = folds.iter().map(|(_, folded)| folded.count).sum();
    if folded > 0 {
        rows.push(other_row(scale, counted(lang, MODEL_FORMS, folded), &folds));
    }
    let total = u64::try_from(shown)
        .unwrap_or(u64::MAX)
        .saturating_add(folded);
    BreakdownList {
        caption: counted(lang, MODEL_FORMS, total),
        rows,
    }
}

fn project_row(scale: Scale, project: &ProjectSpend) -> BreakdownRow {
    let figures = Figures {
        cost_micros: project.cost_usd_micros,
        tokens: project.total_tokens,
        per_mtok: per_mtok_of(
            project.cost_usd_micros,
            project.total_tokens,
            project.partial,
        ),
    };
    let share = if scale.basis == Basis::Cost {
        share_text(scale.lang, project.share_permille)
    } else {
        scale.share(project.cost_usd_micros, project.total_tokens)
    };
    let name = project_label(scale.lang, project.project.as_deref());
    BreakdownRow {
        mark: RowMark::Folder,
        name: middle_ellipsis(&name, PROJECT_NAME_CHARS),
        detail: None,
        share,
        value: scale.value(figures),
        bar: project
            .by_provider
            .iter()
            .map(|part| {
                scale.part(
                    Some(part.provider.clone()),
                    part.cost_usd_micros,
                    part.total_tokens,
                )
            })
            .collect(),
        tooltip: format!("{name}\n{}", scale.tooltip(figures)),
    }
}

#[must_use]
pub fn project_list(
    lang: Lang,
    unit: SpendUnit,
    basis: Basis,
    period: &PeriodSpend,
) -> Option<BreakdownList> {
    let projects = period.projects.as_ref()?;
    let scale = Scale::new(lang, unit, basis, period);
    let mut rows: Vec<BreakdownRow> = projects
        .iter()
        .map(|project| project_row(scale, project))
        .collect();
    let mut total = u64::try_from(projects.len()).unwrap_or(u64::MAX);
    if let Some(other) = &period.projects_other {
        let mut folded = Folded::default();
        folded.add(
            other.count,
            other.cost_usd_micros,
            other.total_tokens,
            other.partial,
        );
        let mut row = other_row(
            scale,
            counted(lang, PROJECT_FORMS, other.count),
            &[(None, folded)],
        );
        row.mark = RowMark::Folder;
        rows.push(row);
        total = total.saturating_add(other.count);
    }
    Some(BreakdownList {
        caption: counted(lang, PROJECT_FORMS, total),
        rows,
    })
}

#[cfg(test)]
#[path = "breakdown_tests.rs"]
mod tests;
