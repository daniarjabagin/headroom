use headroom_core::account::ProviderId;

use super::*;
use crate::state::payload::{ModelView, OtherModelsView, TokensView};
use crate::testing::{CLAUDE, CODEX, descriptor_of};

fn model(name: &str, tokens: u64, cost: i64, partial: bool) -> ModelView {
    ModelView {
        model: name.into(),
        total_tokens: tokens,
        cost_usd_micros: cost,
        partial,
    }
}

fn totals(tokens: u64, cost: i64, partial: bool) -> TotalsView {
    TotalsView {
        tokens: TokensView {
            input: tokens,
            cache_read: 0,
            cache_write: 0,
            output: 0,
            reasoning: 0,
            total: tokens,
        },
        cost_usd_micros: cost,
        partial,
        unpriced_tokens: 0,
        unpriced_models: Vec::new(),
        models: Vec::new(),
        models_other: None,
    }
}

fn usage(provider: ProviderId, home: &str, today: TotalsView) -> UsageView {
    let provider_name = descriptor_of(&provider).display_name.to_owned();
    UsageView {
        provider,
        provider_name,
        usage_home: home.into(),
        today,
        yesterday: totals(0, 0, false),
        last_30_days: totals(10, 20, false),
        daily: Vec::new(),
    }
}

fn provider_rows(period: &PeriodSpendView) -> Vec<(ProviderId, i64, u64, bool)> {
    period
        .by_provider
        .iter()
        .map(|p| {
            (
                p.provider.clone(),
                p.cost_usd_micros,
                p.total_tokens,
                p.partial,
            )
        })
        .collect()
}

#[test]
fn homes_of_one_provider_are_summed() {
    let entries = [
        usage(CODEX, "~/.codex", totals(1_000, 2_000, false)),
        usage(CODEX, "/srv/codex", totals(500, 1_000, false)),
        usage(CLAUDE, "~/.claude", totals(4_000, 9_000, false)),
    ];
    let today = spend(&entries).today;
    assert_eq!(today.cost_usd_micros, 12_000);
    assert_eq!(today.total_tokens, 5_500);
    assert!(!today.partial);
    assert_eq!(
        provider_rows(&today),
        [(CLAUDE, 9_000, 4_000, false), (CODEX, 3_000, 1_500, false),]
    );
    let names: Vec<_> = today
        .by_provider
        .iter()
        .map(|p| p.provider_name.as_str())
        .collect();
    assert_eq!(names, ["Claude", "Codex"]);
}

#[test]
fn equal_costs_are_ordered_by_provider_name() {
    let entries = [
        usage(CODEX, "~/.codex", totals(1, 7, false)),
        usage(CLAUDE, "~/.claude", totals(2, 7, false)),
    ];
    let names: Vec<_> = spend(&entries)
        .today
        .by_provider
        .iter()
        .map(|p| p.provider.clone())
        .collect();
    assert_eq!(names, [CLAUDE, CODEX]);
}

#[test]
fn partial_propagates_to_provider_and_period() {
    let entries = [
        usage(CODEX, "~/.codex", totals(1_000, 2_000, false)),
        usage(CODEX, "/srv/codex", totals(300, 0, true)),
        usage(CLAUDE, "~/.claude", totals(10, 30, false)),
    ];
    let result = spend(&entries);
    assert!(result.today.partial);
    assert_eq!(
        provider_rows(&result.today),
        [(CODEX, 2_000, 1_300, true), (CLAUDE, 30, 10, false),]
    );
    assert!(!result.last_30_days.partial);
}

#[test]
fn providers_without_usage_in_a_period_are_left_out() {
    let entries = [usage(CODEX, "~/.codex", totals(1_000, 2_000, false))];
    let result = spend(&entries);
    assert_eq!(result.yesterday, PeriodSpendView::default());
    assert_eq!(result.last_30_days.by_provider.len(), 1);
    assert_eq!(result.last_30_days.cost_usd_micros, 20);
}

#[test]
fn no_usage_gives_empty_periods() {
    assert_eq!(spend(&[]), SpendView::default());
}

#[test]
fn provider_models_are_merged_across_homes_and_ranked() {
    let mut main = totals(1_000, 2_000, false);
    main.models = vec![
        model("gpt-5.5", 800, 1_800, false),
        model("gpt-5.5-mini", 200, 200, false),
    ];
    let mut spare = totals(700, 300, true);
    spare.models = vec![
        model("gpt-5.5-mini", 400, 300, false),
        model("mystery", 300, 0, true),
        model("free", 300, 0, false),
    ];
    let entries = [
        usage(CODEX, "~/.codex", main),
        usage(CODEX, "/srv/codex", spare),
    ];
    let today = spend(&entries).today;
    assert_eq!(
        today.by_provider[0].models,
        [
            model("gpt-5.5", 800, 1_800, false),
            model("gpt-5.5-mini", 600, 500, false),
            model("free", 300, 0, false),
            model("mystery", 300, 0, true),
        ]
    );
}

#[test]
fn provider_models_keep_the_top_five_and_fold_the_rest_after_merging() {
    let mut main = totals(60, 2_100, false);
    main.models = vec![
        model("a", 10, 600, false),
        model("b", 10, 500, false),
        model("c", 10, 400, false),
        model("d", 10, 300, false),
        model("e", 10, 200, false),
        model("f", 10, 100, false),
    ];
    let mut spare = totals(15, 500, true);
    spare.models = vec![
        model("f", 5, 450, false),
        model("g", 7, 0, true),
        model("h", 3, 50, false),
    ];
    let entries = [
        usage(CODEX, "~/.codex", main),
        usage(CODEX, "/srv/codex", spare),
    ];
    let provider = spend(&entries).today.by_provider.remove(0);
    let names: Vec<_> = provider.models.iter().map(|m| m.model.as_str()).collect();
    assert_eq!(names, ["a", "f", "b", "c", "d"]);
    assert_eq!(provider.models[1], model("f", 15, 550, false));
    assert_eq!(
        provider.models_other,
        Some(OtherModelsView {
            count: 3,
            total_tokens: 20,
            cost_usd_micros: 250,
            partial: true,
        })
    );
}
