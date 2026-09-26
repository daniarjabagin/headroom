use crate::payload::{
    ModelUsage, OtherModels, OtherProjects, PeriodSpend, ProjectProvider, ProjectSpend,
    ProviderSpend,
};

pub(super) fn model(name: &str, cost: i64, tokens: u64) -> ModelUsage {
    ModelUsage {
        model: name.into(),
        total_tokens: tokens,
        cost_usd_micros: cost,
        partial: false,
        cost_per_mtok_usd_micros: None,
    }
}

pub(super) fn provider(
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
            cost_per_mtok_usd_micros: Some(784_211),
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

pub(super) fn period() -> PeriodSpend {
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
            cost_per_mtok_usd_micros: Some(803_333),
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
            cost_per_mtok_usd_micros: None,
        }),
        models: None,
        models_other: None,
    }
}
