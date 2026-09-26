use super::*;
use crate::popup_model::breakdown::RowMark;
use crate::popup_model::spend_fixture::{model, period};

fn listed(provider: &str, name: &str, cost: i64, tokens: u64) -> ProviderModelUsage {
    ProviderModelUsage {
        provider: provider.into(),
        provider_name: provider.into(),
        usage: model(name, cost, tokens),
    }
}

fn daemon_period(rate: Option<i64>) -> PeriodSpend {
    PeriodSpend {
        models: Some(vec![
            listed("claude", "Opus 4.6", 41_200_000, 38_400_000),
            listed("codex", "GPT-5-Codex", 38_600_000, 50_000_000),
            listed("claude", "Sonnet 4.6", 19_800_000, 52_100_000),
        ]),
        models_other: Some(OtherModels {
            count: 5,
            total_tokens: 68_500_000,
            cost_usd_micros: 28_050_000,
            partial: false,
            cost_per_mtok_usd_micros: rate,
        }),
        ..period()
    }
}

fn names(list: &BreakdownList) -> Vec<&str> {
    list.rows.iter().map(|row| row.name.as_str()).collect()
}

#[test]
fn daemon_models_keep_their_order_and_fold() {
    let list = model_list(Lang::En, SpendUnit::Cost, Basis::Cost, &daemon_period(None));
    assert_eq!(
        names(&list),
        ["Opus 4.6", "GPT-5-Codex", "Sonnet 4.6", "Other"]
    );
    assert_eq!(list.caption, "8 models");
    assert_eq!(list.rows[0].mark, RowMark::Dot("claude".into()));
    assert_eq!(list.rows[0].share, "32.2%");
    let other = &list.rows[3];
    assert_eq!(other.mark, RowMark::None);
    assert_eq!(other.detail.as_deref(), Some("5 models"));
    assert_eq!(other.value, "$28.05");
    assert_eq!(other.bar.len(), 1);
    assert_eq!(other.bar[0].provider, None);
}

#[test]
fn the_daemon_other_rate_is_shown_as_is() {
    let rated = daemon_period(Some(409_489));
    let list = model_list(Lang::En, SpendUnit::CostPerMtok, Basis::Cost, &rated);
    assert_eq!(list.rows[3].value, "$0.41");
    let unrated = daemon_period(None);
    let list = model_list(Lang::En, SpendUnit::CostPerMtok, Basis::Cost, &unrated);
    assert_eq!(list.rows[3].value, "—");
}

#[test]
fn daemon_models_follow_the_token_basis() {
    let list = model_list(
        Lang::En,
        SpendUnit::Tokens,
        Basis::Tokens,
        &daemon_period(None),
    );
    assert_eq!(
        names(&list),
        ["Sonnet 4.6", "GPT-5-Codex", "Opus 4.6", "Other"]
    );
    assert_eq!(list.rows[0].value, "52.1M");
}

#[test]
fn older_daemons_list_every_provider_model_and_their_others() {
    let list = model_list(Lang::En, SpendUnit::Cost, Basis::Cost, &period());
    assert_eq!(
        names(&list),
        [
            "Opus 4.6",
            "GPT-5-Codex",
            "Sonnet 4.6",
            "Grok 4",
            "GPT-5",
            "Haiku 4.5",
            "Other"
        ]
    );
    assert_eq!(list.caption, "8 models");
    let other = &list.rows[6];
    assert_eq!(other.detail.as_deref(), Some("2 models"));
    assert_eq!(other.value, "$1.49");
    assert_eq!(other.bar.len(), 1);
    assert_eq!(other.bar[0].provider.as_deref(), Some("claude"));
}

#[test]
fn older_daemons_add_the_provider_others_without_a_rate() {
    let mut older = period();
    older.by_provider[1].models_other = Some(OtherModels {
        count: 3,
        total_tokens: 1_000_000,
        cost_usd_micros: 510_000,
        partial: true,
        cost_per_mtok_usd_micros: Some(510_000),
    });
    let list = model_list(Lang::En, SpendUnit::CostPerMtok, Basis::Cost, &older);
    let other = list.rows.last().unwrap();
    assert_eq!(other.detail.as_deref(), Some("5 models"));
    assert_eq!(other.value, "—");
    let providers: Vec<Option<&str>> = other
        .bar
        .iter()
        .map(|part| part.provider.as_deref())
        .collect();
    assert_eq!(providers, [Some("claude"), Some("codex")]);
    let mut single = period();
    single.by_provider.truncate(1);
    let list = model_list(Lang::En, SpendUnit::CostPerMtok, Basis::Cost, &single);
    assert_eq!(list.rows.last().unwrap().value, "—");
}
