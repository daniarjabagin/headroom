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
