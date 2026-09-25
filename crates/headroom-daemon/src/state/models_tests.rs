use super::*;

fn row(name: &str, tokens: u64, cost: i64, partial: bool) -> ModelView {
    ModelView {
        model: name.into(),
        total_tokens: tokens,
        cost_usd_micros: cost,
        partial,
        cost_per_mtok_usd_micros: None,
    }
}

fn ranked(count: usize) -> Vec<ModelView> {
    (0..count)
        .map(|n| {
            let rank = u64::try_from(count - n).unwrap();
            row(
                &format!("m{n}"),
                rank * 10,
                i64::try_from(rank).unwrap(),
                false,
            )
        })
        .collect()
}

#[test]
fn five_or_fewer_models_have_no_other_bucket() {
    for count in 0..=TOP_MODELS {
        let (models, other) = top_models(ranked(count));
        assert_eq!(models, ranked(count));
        assert_eq!(other, None);
    }
}

#[test]
fn models_beyond_the_top_five_are_summed_into_other() {
    let mut rows = ranked(5);
    rows.extend([
        row("x", 7, 3, false),
        row("y", 11, 2, true),
        row("z", 1, 0, false),
    ]);
    let (models, other) = top_models(rows);
    assert_eq!(models, ranked(5));
    assert_eq!(
        other,
        Some(OtherModelsView {
            count: 3,
            total_tokens: 19,
            cost_usd_micros: 5,
            partial: true,
        })
    );
}

#[test]
fn other_bucket_is_exact_for_a_single_extra_model() {
    let mut rows = ranked(5);
    rows.push(row("tail", 42, 0, false));
    let (_, other) = top_models(rows);
    assert_eq!(
        other,
        Some(OtherModelsView {
            count: 1,
            total_tokens: 42,
            cost_usd_micros: 0,
            partial: false,
        })
    );
}
