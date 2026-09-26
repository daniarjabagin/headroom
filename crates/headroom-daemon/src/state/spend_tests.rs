use std::path::PathBuf;

use headroom_core::event::UsageEvent;
use headroom_core::usage::aggregate;
use jiff::tz::TimeZone;

use super::*;
use crate::catalog::ProviderCatalog;
use crate::home::HomeDisplay;
use crate::state::payload::{ModelView, OtherModelsView};
use crate::testing::{CLAUDE, CODEX, FlatPrices, catalog, event, ts};

const NOW: &str = "2026-09-23T10:00:00Z";
const TODAY: &str = "2026-09-23T08:00:00Z";
const LAST_WEEK: &str = "2026-09-18T08:00:00Z";
const LAST_MONTH: &str = "2026-09-01T08:00:00Z";

struct Fixture {
    homes: Vec<(UsageHome, UsageSummary)>,
    display: HomeDisplay,
    catalog: ProviderCatalog,
}

impl Fixture {
    fn new() -> Fixture {
        Fixture {
            homes: Vec::new(),
            display: HomeDisplay::new(Some(PathBuf::from("/home/ada"))),
            catalog: catalog(),
        }
    }

    fn home(mut self, provider: &ProviderId, dir: &str, events: &[UsageEvent]) -> Fixture {
        let home = UsageHome {
            provider: provider.clone(),
            home: PathBuf::from(dir),
        };
        let summary = aggregate(events, &FlatPrices, &TimeZone::UTC, ts(NOW));
        self.homes.push((home, summary));
        self
    }

    fn spend(&self) -> SpendView {
        let listed: Vec<HomeSummary<'_>> = self.homes.iter().map(|(h, s)| (h, s)).collect();
        let ctx = AssembleContext {
            now: ts(NOW),
            tz: &TimeZone::UTC,
            homes: &self.display,
            catalog: &self.catalog,
        };
        spend(&listed, &ctx)
    }
}

fn used(key: &str, model: &str, tokens: u64) -> UsageEvent {
    event(key, TODAY, model, tokens, 0)
}

fn model(name: &str, tokens: u64, cost: i64, partial: bool) -> ModelView {
    ModelView {
        model: name.into(),
        total_tokens: tokens,
        cost_usd_micros: cost,
        partial,
        cost_per_mtok_usd_micros: (!partial).then_some(2_000_000),
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
    let spend = Fixture::new()
        .home(&CODEX, "/home/ada/.codex", &[used("a", "gpt-5.5", 1_000)])
        .home(&CODEX, "/srv/codex", &[used("b", "gpt-5.5", 500)])
        .home(
            &CLAUDE,
            "/home/ada/.claude",
            &[used("c", "claude-opus", 4_000)],
        )
        .spend();
    let today = spend.today;
    assert_eq!(today.cost_usd_micros, 11_000);
    assert_eq!(today.total_tokens, 5_500);
    assert!(!today.partial);
    assert_eq!(
        provider_rows(&today),
        [(CLAUDE, 8_000, 4_000, false), (CODEX, 3_000, 1_500, false)]
    );
    let names: Vec<_> = today
        .by_provider
        .iter()
        .map(|p| p.provider_name.as_str())
        .collect();
    assert_eq!(names, ["Claude", "Codex"]);
}

#[test]
fn equal_costs_are_ordered_by_provider_id() {
    let spend = Fixture::new()
        .home(&CODEX, "/home/ada/.codex", &[used("a", "gpt-5.5", 7)])
        .home(&CLAUDE, "/home/ada/.claude", &[used("b", "claude-opus", 7)])
        .spend();
    let ids: Vec<_> = spend
        .today
        .by_provider
        .iter()
        .map(|p| p.provider.clone())
        .collect();
    assert_eq!(ids, [CLAUDE, CODEX]);
}

#[test]
fn partial_propagates_to_provider_and_period() {
    let spend = Fixture::new()
        .home(&CODEX, "/home/ada/.codex", &[used("a", "gpt-5.5", 1_000)])
        .home(&CODEX, "/srv/codex", &[used("b", "unknown", 300)])
        .home(
            &CLAUDE,
            "/home/ada/.claude",
            &[event("c", LAST_MONTH, "x", 10, 0)],
        )
        .spend();
    assert!(spend.today.partial);
    assert_eq!(provider_rows(&spend.today), [(CODEX, 2_000, 1_300, true)]);
    assert_eq!(spend.today.cost_per_mtok_usd_micros, Some(2_000_000));
    assert_eq!(
        spend.today.by_provider[0].cost_per_mtok_usd_micros,
        Some(2_000_000)
    );
    assert_eq!(spend.last_30_days.by_provider.len(), 2);
    assert_eq!(spend.yesterday, PeriodSpendView::default());
}

#[test]
fn cost_per_mtok_leaves_unpriced_tokens_out_and_is_null_without_priced_tokens() {
    let reported = UsageEvent {
        reported_cost: Some(headroom_core::units::MicroUsd(1)),
        ..used("r", "gpt-5.5", 3)
    };
    let spend = Fixture::new()
        .home(
            &CODEX,
            "/home/ada/.codex",
            &[used("a", "unknown", 50), reported],
        )
        .home(&CLAUDE, "/home/ada/.claude", &[used("b", "unknown", 10)])
        .spend();
    let today = &spend.today;
    assert_eq!(today.cost_per_mtok_usd_micros, Some(333_333));
    let codex = &today.by_provider[0];
    assert_eq!(codex.provider, CODEX);
    assert_eq!(codex.cost_per_mtok_usd_micros, Some(333_333));
    assert_eq!(codex.models[0].cost_per_mtok_usd_micros, Some(333_333));
    assert_eq!(codex.models[1].cost_per_mtok_usd_micros, None);
    assert_eq!(today.by_provider[1].cost_per_mtok_usd_micros, None);
    assert_eq!(spend.yesterday.cost_per_mtok_usd_micros, None);
}

#[test]
fn the_last_seven_days_sit_between_today_and_thirty_days() {
    let events = [
        used("a", "gpt-5.5", 10),
        event("b", LAST_WEEK, "gpt-5.5", 20, 0),
        event("c", "2026-09-17T08:00:00Z", "gpt-5.5", 40, 0),
        event("d", "2026-09-16T23:59:59Z", "gpt-5.5", 80, 0),
    ];
    let spend = Fixture::new()
        .home(&CODEX, "/home/ada/.codex", &events)
        .spend();
    assert_eq!(spend.today.total_tokens, 10);
    assert_eq!(spend.last_7_days.total_tokens, 70);
    assert_eq!(spend.last_7_days.cost_usd_micros, 140);
    assert_eq!(spend.last_30_days.total_tokens, 150);
}

#[test]
fn no_usage_gives_empty_periods() {
    assert_eq!(Fixture::new().spend(), SpendView::default());
}

#[test]
fn provider_models_are_merged_across_homes_and_ranked() {
    let spend = Fixture::new()
        .home(
            &CODEX,
            "/home/ada/.codex",
            &[used("a", "gpt-5.5", 900), used("b", "gpt-5.5-mini", 200)],
        )
        .home(
            &CODEX,
            "/srv/codex",
            &[used("c", "gpt-5.5-mini", 400), used("d", "unknown", 300)],
        )
        .spend();
    assert_eq!(
        spend.today.by_provider[0].models,
        [
            model("gpt-5.5", 900, 1_800, false),
            model("gpt-5.5-mini", 600, 1_200, false),
            model("unknown", 300, 0, true),
        ]
    );
}

#[test]
fn provider_models_keep_the_top_five_and_fold_the_rest_after_merging() {
    let main: Vec<UsageEvent> = [
        ("a", 60),
        ("b", 50),
        ("c", 40),
        ("d", 30),
        ("e", 20),
        ("f", 10),
    ]
    .iter()
    .map(|(name, tokens)| used(name, name, *tokens))
    .collect();
    let spare = [
        used("f2", "f", 45),
        used("g", "unknown", 7),
        used("h", "h", 3),
    ];
    let spend = Fixture::new()
        .home(&CODEX, "/home/ada/.codex", &main)
        .home(&CODEX, "/srv/codex", &spare)
        .spend();
    let provider = &spend.today.by_provider[0];
    let names: Vec<_> = provider.models.iter().map(|m| m.model.as_str()).collect();
    assert_eq!(names, ["a", "f", "b", "c", "d"]);
    assert_eq!(provider.models[1], model("f", 55, 110, false));
    assert_eq!(
        provider.models_other,
        Some(OtherModelsView {
            count: 3,
            total_tokens: 30,
            cost_usd_micros: 46,
            partial: true,
            cost_per_mtok_usd_micros: Some(2_000_000),
        })
    );
}

fn reconciled(period: &PeriodSpendView) -> (i64, u64) {
    let other = period.models_other.clone().unwrap_or_default();
    period.models.iter().fold(
        (other.cost_usd_micros, other.total_tokens),
        |(cost, tokens), m| {
            (
                cost + m.model.cost_usd_micros,
                tokens + m.model.total_tokens,
            )
        },
    )
}

#[test]
fn merged_models_span_providers_and_reconcile_with_the_period_totals() {
    let codex: Vec<UsageEvent> = [("a", 90), ("b", 70), ("shared", 50), ("unknown", 400)]
        .iter()
        .map(|(name, tokens)| used(name, name, *tokens))
        .collect();
    let claude = [
        used("c1", "shared", 60),
        used("c2", "c", 30),
        used("c3", "d", 20),
        used("c4", "unknown", 5),
        event("c5", LAST_MONTH, "old", 1_000, 0),
    ];
    let spend = Fixture::new()
        .home(&CODEX, "/home/ada/.codex", &codex)
        .home(&CODEX, "/srv/codex", &[used("s", "b", 15)])
        .home(&CLAUDE, "/home/ada/.claude", &claude)
        .spend();
    let today = &spend.today;
    let rows: Vec<_> = today
        .models
        .iter()
        .map(|m| (m.provider.clone(), m.model.model.as_str()))
        .collect();
    assert_eq!(
        rows,
        [
            (CODEX, "a"),
            (CODEX, "b"),
            (CLAUDE, "shared"),
            (CODEX, "shared"),
            (CLAUDE, "c"),
        ]
    );
    assert_eq!(today.models[1].model, model("b", 85, 170, false));
    assert_eq!(today.models[2].provider_name, "Claude");
    assert_eq!(
        today.models_other,
        Some(OtherModelsView {
            count: 3,
            total_tokens: 425,
            cost_usd_micros: 40,
            partial: true,
            cost_per_mtok_usd_micros: Some(2_000_000),
        })
    );
    assert_eq!(
        reconciled(today),
        (today.cost_usd_micros, today.total_tokens)
    );
    let month = &spend.last_30_days;
    assert_eq!(
        reconciled(month),
        (month.cost_usd_micros, month.total_tokens)
    );
    assert_eq!(spend.yesterday.models, []);
    assert_eq!(spend.yesterday.models_other, None);
}
