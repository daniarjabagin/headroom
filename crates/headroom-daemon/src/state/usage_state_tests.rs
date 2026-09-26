use std::path::PathBuf;

use headroom_core::account::AccountId;
use headroom_core::usage::aggregate;
use jiff::tz::TimeZone;

use super::tests::{assemble_sample, claude_usage, codex_usage, sample_model};
use crate::home::UsageHome;
use crate::testing::{CLAUDE, CODEX, FlatPrices, event, ts};

const NOW: &str = "2026-09-23T10:00:00Z";

#[test]
fn daily_trend_is_dense_over_thirty_days() {
    let payload = assemble_sample(&sample_model());
    let daily = &payload
        .usage
        .iter()
        .find(|u| u.provider == CODEX)
        .unwrap()
        .daily;
    assert_eq!(daily.len(), 30);
    assert_eq!(daily[0].date.to_string(), "2026-08-25");
    assert_eq!(daily[29].date.to_string(), "2026-09-23");
    assert_eq!(daily[29].total_tokens, 1_200);
    assert!(daily[28].partial);
    assert_eq!(daily[27].total_tokens, 0);
}

#[test]
fn usage_of_undiscovered_homes_is_omitted() {
    let mut model = sample_model();
    let stray = UsageHome {
        provider: CLAUDE,
        home: PathBuf::from("/nowhere"),
    };
    model.usage.insert(stray, codex_usage());
    model.usage_homes.retain(|home| home.provider != CODEX);
    let homes: Vec<_> = assemble_sample(&model)
        .usage
        .iter()
        .map(|u| u.usage_home.clone())
        .collect();
    assert_eq!(homes, ["~/.claude"]);
}

#[test]
fn usage_homes_without_accounts_are_listed_and_spent() {
    let mut model = sample_model();
    let api_key = UsageHome {
        provider: CLAUDE,
        home: PathBuf::from("/home/ada/.claude-api"),
    };
    model.usage_homes.insert(api_key.clone());
    model.usage.insert(api_key, claude_usage());
    model
        .accounts
        .retain(|a| a.id() != &AccountId("claude:main".into()));
    let payload = assemble_sample(&model);
    let homes: Vec<_> = payload
        .usage
        .iter()
        .map(|u| u.usage_home.as_str())
        .collect();
    assert_eq!(homes, ["~/.codex", "~/.claude", "~/.claude-api"]);
    assert_eq!(payload.spend.today.total_tokens, 11_200);
    assert_eq!(payload.spend.today.cost_usd_micros, 22_400);
}

#[test]
fn usage_totals_carry_models_per_period() {
    let payload = assemble_sample(&sample_model());
    let codex = &payload.usage[0];
    let names = |models: &[super::payload::ModelView]| -> Vec<String> {
        models.iter().map(|m| m.model.clone()).collect()
    };
    assert_eq!(names(&codex.today.models), ["gpt-5.5"]);
    assert_eq!(names(&codex.yesterday.models), ["gpt-5.5", "unknown"]);
    assert!(codex.yesterday.models[1].partial);
    let spend_codex = &payload.spend.last_30_days.by_provider[1];
    assert_eq!(spend_codex.provider, CODEX);
    assert_eq!(spend_codex.models[0].total_tokens, 1_800);
}

#[test]
fn usage_models_are_cut_to_the_top_five_after_spend_is_merged() {
    let mut model = sample_model();
    let names = ["m1", "m2", "m3", "m4", "m5", "m6", "unknown"];
    let events: Vec<_> = names
        .iter()
        .zip(1_u64..)
        .map(|(name, n)| event(name, "2026-09-23T08:00:00Z", name, n * 100, 0))
        .collect();
    let home = model.usage_homes.iter().next().unwrap().clone();
    let summary = aggregate(&events, &FlatPrices, &TimeZone::UTC, ts(NOW));
    model.usage.insert(home.clone(), summary);
    let payload = assemble_sample(&model);
    let usage = payload
        .usage
        .iter()
        .find(|u| u.provider == home.provider)
        .unwrap();
    let today = &usage.today;
    assert_eq!(today.models.len(), 5);
    assert_eq!(today.models[0].model, "m6");
    let other = today.models_other.as_ref().unwrap();
    assert_eq!((other.count, other.total_tokens), (2, 800));
    assert_eq!(other.cost_usd_micros, 200);
    assert!(other.partial);
    assert_eq!(other.cost_per_mtok_usd_micros, Some(2_000_000));
    let listed: u64 = today.models.iter().map(|m| m.total_tokens).sum();
    assert_eq!(listed + other.total_tokens, today.tokens.total);
    let spend = payload
        .spend
        .today
        .by_provider
        .iter()
        .find(|p| p.provider == home.provider)
        .unwrap();
    assert_eq!(spend.models, today.models);
    assert_eq!(spend.models_other.as_ref(), Some(other));
    assert_eq!(usage.yesterday.models_other, None);
}
