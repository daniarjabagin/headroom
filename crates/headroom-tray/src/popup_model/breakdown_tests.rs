use super::*;
use crate::payload::{OtherModels, OtherProjects, ProjectProvider};
use crate::popup_model::model_card::model_card;

fn model(name: &str, cost: i64, tokens: u64) -> ModelUsage {
    ModelUsage {
        model: name.into(),
        total_tokens: tokens,
        cost_usd_micros: cost,
        partial: false,
        cost_per_mtok_usd_micros: None,
    }
}

fn provider(
    id: &str,
    name: &str,
    models: Vec<ModelUsage>,
    other: Option<OtherModels>,
) -> ProviderSpend {
    let cost = models.iter().map(|m| m.cost_usd_micros).sum::<i64>()
        + other.as_ref().map_or(0, |o| o.cost_usd_micros);
    let tokens = models.iter().map(|m| m.total_tokens).sum::<u64>()
        + other.as_ref().map_or(0, |o| o.total_tokens);
    ProviderSpend {
        provider: id.into(),
        provider_name: name.into(),
        cost_usd_micros: cost,
        total_tokens: tokens,
        partial: false,
        models,
        models_other: other,
        cost_per_mtok_usd_micros: None,
    }
}

fn providers() -> Vec<ProviderSpend> {
    let claude = provider(
        "claude",
        "Claude",
        vec![
            model("Opus 4.6", 41_200_000, 38_400_000),
            model("Sonnet 4.6", 19_800_000, 52_100_000),
            model("Haiku 4.5", 3_960_000, 31_700_000),
        ],
        Some(OtherModels {
            count: 2,
            total_tokens: 1_900_000,
            cost_usd_micros: 1_490_000,
            partial: false,
        }),
    );
    let codex = provider(
        "codex",
        "Codex",
        vec![
            model("GPT-5-Codex", 38_600_000, 50_000_000),
            model("GPT-5", 10_300_000, 20_000_000),
        ],
        None,
    );
    let grok = provider(
        "grok",
        "Grok",
        vec![model("Grok 4", 12_300_000, 14_900_000)],
        None,
    );
    vec![claude, codex, grok]
}

fn part(id: &str, name: &str, cost: i64, tokens: u64) -> ProjectProvider {
    ProjectProvider {
        provider: id.into(),
        provider_name: name.into(),
        cost_usd_micros: cost,
        total_tokens: tokens,
    }
}

fn period() -> PeriodSpend {
    let by_provider = providers();
    PeriodSpend {
        cost_usd_micros: by_provider.iter().map(|p| p.cost_usd_micros).sum(),
        total_tokens: by_provider.iter().map(|p| p.total_tokens).sum(),
        partial: false,
        by_provider,
        cost_per_mtok_usd_micros: None,
        projects: Some(vec![ProjectSpend {
            project: Some("~/work/headroom".into()),
            cost_usd_micros: 48_200_000,
            total_tokens: 60_000_000,
            partial: false,
            share_permille: 378,
            by_provider: vec![
                part("claude", "Claude", 30_000_000, 40_000_000),
                part("codex", "Codex", 18_200_000, 20_000_000),
            ],
        }]),
        projects_other: Some(OtherProjects {
            count: 7,
            cost_usd_micros: 79_450_000,
            total_tokens: 148_900_000,
            partial: false,
            share_permille: 622,
        }),
    }
}

#[test]
fn models_rank_across_providers_and_fold_the_rest() {
    let list = model_list(Lang::En, SpendUnit::Cost, Basis::Cost, &period());
    let names: Vec<&str> = list.rows.iter().map(|row| row.name.as_str()).collect();
    assert_eq!(
        names,
        [
            "Opus 4.6",
            "GPT-5-Codex",
            "Sonnet 4.6",
            "Grok 4",
            "GPT-5",
            "Other"
        ]
    );
    assert_eq!(list.caption, "8 models");
    let first = &list.rows[0];
    assert_eq!(first.mark, RowMark::Dot("claude".into()));
    assert_eq!(first.share, "32.2%");
    assert_eq!(first.value, "$41.20");
    let other = &list.rows[5];
    assert_eq!(other.detail.as_deref(), Some("3 models"));
    assert_eq!(other.value, "$5.45");
    assert_eq!(other.bar.len(), 1);
    assert_eq!(other.bar[0].provider.as_deref(), Some("claude"));
}

#[test]
fn model_values_follow_the_unit() {
    let list = model_list(Lang::En, SpendUnit::Tokens, Basis::Tokens, &period());
    assert_eq!(list.rows[0].name, "Sonnet 4.6");
    assert_eq!(list.rows[0].value, "52.1M");
}

#[test]
fn projects_use_the_daemon_share_and_split_bars() {
    let list = project_list(Lang::En, SpendUnit::Cost, Basis::Cost, &period()).unwrap();
    assert_eq!(list.caption, "8 projects");
    let first = &list.rows[0];
    assert_eq!(first.name, "~/work/headroom");
    assert_eq!(first.share, "37.8%");
    assert_eq!(first.bar.len(), 2);
    let other = &list.rows[1];
    assert_eq!(other.mark, RowMark::Folder);
    assert_eq!(other.detail.as_deref(), Some("7 projects"));
    assert_eq!(other.bar[0].provider, None);
    let mut none = period();
    none.projects = None;
    assert!(project_list(Lang::En, SpendUnit::Cost, Basis::Cost, &none).is_none());
}

#[test]
fn the_model_card_lists_one_provider() {
    let period = period();
    let card = model_card(
        Lang::En,
        SpendUnit::Cost,
        Basis::Cost,
        "Last 30 days",
        &period.by_provider[0],
    )
    .unwrap();
    assert_eq!(card.title, "Claude · Last 30 days");
    assert_eq!(card.total, "$66.45");
    assert_eq!(card.lines.len(), 4);
    assert_eq!(card.lines[0].share, "62%");
    assert_eq!(card.lines[0].tokens, "38.4M tokens");
    assert_eq!(card.lines[3].name, "Other");
    assert_eq!(card.lines[3].detail.as_deref(), Some("2 models"));
    assert!(card.folded);
    let bare = provider("x", "X", Vec::new(), None);
    assert!(model_card(Lang::En, SpendUnit::Cost, Basis::Cost, "t", &bare).is_none());
}
