use headroom_core::tokens::TokenCounts;
use headroom_core::units::{MicroUsd, Tokens};

use super::*;

fn usage(name: &str, tokens: u64, cost: i64, unpriced: u64) -> ModelUsage {
    let mut totals = UsageTotals {
        tokens: TokenCounts {
            input: Tokens(tokens),
            ..TokenCounts::default()
        },
        cost: MicroUsd(cost),
        unpriced_tokens: Tokens(unpriced),
        ..UsageTotals::default()
    };
    if unpriced > 0 {
        totals.unpriced_models.insert(name.into());
    }
    ModelUsage {
        model: name.into(),
        totals,
    }
}

fn ranked(count: usize) -> Vec<ModelUsage> {
    (0..count)
        .map(|n| {
            let rank = u64::try_from(count - n).unwrap();
            usage(&format!("m{n}"), rank * 10, i64::try_from(rank).unwrap(), 0)
        })
        .collect()
}

fn views(rows: &[ModelUsage]) -> Vec<ModelView> {
    rows.iter().map(model_view).collect()
}

#[test]
fn five_or_fewer_models_have_no_other_bucket() {
    for count in 0..=TOP_MODELS {
        let (models, other) = top_models(&ranked(count));
        assert_eq!(models, views(&ranked(count)));
        assert_eq!(other, None);
    }
}

#[test]
fn models_beyond_the_top_five_are_summed_into_other() {
    let mut rows = ranked(5);
    rows.extend([
        usage("x", 7, 3, 0),
        usage("y", 11, 2, 1),
        usage("z", 1, 0, 0),
    ]);
    let (models, other) = top_models(&rows);
    assert_eq!(models, views(&ranked(5)));
    assert_eq!(
        other,
        Some(OtherModelsView {
            count: 3,
            total_tokens: 19,
            cost_usd_micros: 5,
            partial: true,
            cost_per_mtok_usd_micros: Some(277_778),
        })
    );
}

#[test]
fn other_rate_leaves_unpriced_tokens_out_and_is_null_without_priced_tokens() {
    let mut rows = ranked(5);
    rows.extend([
        usage("x", 500_000, 0, 500_000),
        usage("y", 500_000, 0, 500_000),
    ]);
    let (_, other) = top_models(&rows);
    assert_eq!(other.unwrap().cost_per_mtok_usd_micros, None);
    let mut rows = ranked(5);
    rows.extend([
        usage("x", 500_000, 1_000_000, 0),
        usage("y", 1_000_000, 0, 500_000),
    ]);
    let (_, other) = top_models(&rows);
    assert_eq!(other.unwrap().cost_per_mtok_usd_micros, Some(1_000_000));
}

#[test]
fn a_single_extra_model_is_listed_instead_of_folded() {
    let mut rows = ranked(5);
    rows.push(usage("tail", 42, 0, 0));
    let (models, other) = top_models(&rows);
    assert_eq!(models, views(&rows));
    assert_eq!(other, None);
}

#[test]
fn merged_models_rank_by_cost_then_tokens_then_name() {
    let mut merge = ModelMerge::default();
    merge.add(&[
        usage("b", 10, 5, 0),
        usage("a", 10, 5, 0),
        usage("c", 30, 1, 0),
    ]);
    merge.add(&[usage("c", 5, 9, 0), usage("d", 99, 5, 0)]);
    let names: Vec<_> = merge.ranked().into_iter().map(|m| m.model).collect();
    assert_eq!(names, ["c", "d", "a", "b"]);
}

const CODEX: ProviderId = ProviderId::from_static("codex");
const CLAUDE: ProviderId = ProviderId::from_static("claude");

fn entry(provider: &ProviderId, usage: ModelUsage) -> ProviderModel {
    ProviderModel {
        provider: provider.clone(),
        provider_name: provider.as_str().to_uppercase(),
        usage,
    }
}

#[test]
fn merged_models_keep_same_names_of_different_providers_apart() {
    let (models, other) = merged_top_models(vec![
        entry(&CODEX, usage("shared", 10, 5, 0)),
        entry(&CLAUDE, usage("shared", 10, 5, 0)),
        entry(&CODEX, usage("solo", 1, 9, 0)),
    ]);
    let rows: Vec<_> = models
        .iter()
        .map(|m| {
            (
                m.provider.as_str(),
                m.provider_name.as_str(),
                m.model.model.as_str(),
            )
        })
        .collect();
    assert_eq!(
        rows,
        [
            ("codex", "CODEX", "solo"),
            ("claude", "CLAUDE", "shared"),
            ("codex", "CODEX", "shared"),
        ]
    );
    assert_eq!(models[1].model, model_view(&usage("shared", 10, 5, 0)));
    assert_eq!(other, None);
}

#[test]
fn merged_models_fold_the_tail_with_the_same_limit_and_rate() {
    let mut entries: Vec<_> = ranked(5).into_iter().map(|u| entry(&CODEX, u)).collect();
    entries.push(entry(&CLAUDE, usage("x", 500_000, 1_000_000, 0)));
    entries.push(entry(&CODEX, usage("y", 1_000_000, 0, 500_000)));
    let (models, other) = merged_top_models(entries);
    let names: Vec<_> = models.iter().map(|m| m.model.model.as_str()).collect();
    assert_eq!(names, ["x", "m0", "m1", "m2", "m3"]);
    assert_eq!(
        other,
        Some(OtherModelsView {
            count: 2,
            total_tokens: 1_000_010,
            cost_usd_micros: 1,
            partial: true,
            cost_per_mtok_usd_micros: Some(2),
        })
    );
}

#[test]
fn merged_models_list_a_single_extra_model_instead_of_folding_it() {
    let mut entries: Vec<_> = ranked(5).into_iter().map(|u| entry(&CODEX, u)).collect();
    entries.push(entry(&CLAUDE, usage("tail", 1, 0, 0)));
    let (models, other) = merged_top_models(entries);
    assert_eq!(models.len(), TOP_MODELS + 1);
    assert_eq!(other, None);
}
