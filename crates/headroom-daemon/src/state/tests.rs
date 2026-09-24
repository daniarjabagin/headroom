use std::path::PathBuf;

use headroom_core::account::{AccountId, CredentialOwner, ProviderId};
use headroom_core::pace::Tone;
use headroom_core::provider::ProviderError;
use headroom_core::quota::{Balance, BalanceAmount, LimitsSource, Notice};
use headroom_core::units::MicroUsd;
use headroom_core::usage::aggregate;

use super::payload::{AccountStatus, DataSource};
use super::*;
use crate::home::UsageHome;
use crate::model::{AccountRuntime, RefreshFailure, SnapshotEntry, SnapshotOrigin};
use crate::storage::accounts::AccountRecord;
use crate::testing::{CLAUDE, CODEX, catalog};
use crate::testing::{FlatPrices, account, event, session, snapshot, ts, usage_home_of, weekly};

const NOW: &str = "2026-09-23T10:00:00Z";
const SNAPSHOT_APP_VERSION: &str = "0.0.0-snapshot";

fn record(provider: ProviderId, name: &str, order: i64) -> AccountRecord {
    AccountRecord {
        reference: account(provider, name),
        label: None,
        hidden: false,
        sort_order: order,
        email: None,
        plan: None,
        last_seen: ts(NOW),
        gone: false,
    }
}

fn work_snapshot() -> SnapshotEntry {
    let mut limits = snapshot(
        vec![
            session(55.0, "2026-09-23T12:00:00Z"),
            weekly(30.0, "2026-09-26T10:00:00Z"),
        ],
        "2026-09-23T09:58:00Z",
    );
    limits.balances.push(Balance {
        id: "credits".into(),
        label: "Credits".into(),
        amount: BalanceAmount::Usd(MicroUsd(12_500_000)),
    });
    limits.notices.push(Notice {
        tone: Tone::Warning,
        text: "Weekly limit shared with Codex Cloud".into(),
    });
    SnapshotEntry {
        snapshot: limits,
        origin: SnapshotOrigin::Refreshed,
    }
}

fn claude_snapshot() -> SnapshotEntry {
    let mut limits = snapshot(
        vec![session(92.0, "2026-09-23T10:30:00Z")],
        "2026-09-23T09:00:00Z",
    );
    limits.source = LimitsSource::Live;
    limits.identity.email = Some("ada@claude.example".into());
    SnapshotEntry {
        snapshot: limits,
        origin: SnapshotOrigin::Cache,
    }
}

fn codex_usage() -> headroom_core::usage::UsageSummary {
    let events = [
        event("a", "2026-09-23T08:00:00Z", "gpt-5.5", 1_000, 200),
        event("b", "2026-09-22T08:00:00Z", "gpt-5.5", 500, 100),
        event("c", "2026-09-22T09:00:00Z", "unknown", 10, 5),
    ];
    aggregate(&events, &FlatPrices, &TimeZone::UTC, ts(NOW))
}

fn claude_usage() -> headroom_core::usage::UsageSummary {
    let events = [event(
        "d",
        "2026-09-23T07:00:00Z",
        "claude-opus",
        4_000,
        1_000,
    )];
    aggregate(&events, &FlatPrices, &TimeZone::UTC, ts(NOW))
}

fn sample_model() -> Model {
    let mut work = record(CODEX, "work", 0);
    work.label = Some("Work".into());
    let claude = record(CLAUDE, "main", 1);
    let mut hidden = record(CODEX, "hidden", 2);
    hidden.hidden = true;
    hidden.reference.home = PathBuf::from("/srv/codex");
    let mut gone = record(CODEX, "gone", 3);
    gone.gone = true;
    let mut model = Model {
        accounts: vec![work.clone(), claude.clone(), hidden, gone],
        ..Model::default()
    };
    model.snapshots.insert(work.id().clone(), work_snapshot());
    model
        .snapshots
        .insert(claude.id().clone(), claude_snapshot());
    model.runtime.insert(
        claude.id().clone(),
        AccountRuntime {
            failure: Some(RefreshFailure::Provider(ProviderError::SignInExpired)),
            failures: 1,
            ..AccountRuntime::default()
        },
    );
    model.runtime.insert(
        work.id().clone(),
        AccountRuntime {
            last_attempt: Some(ts("2026-09-23T09:58:00Z")),
            next_refresh_at: Some(ts("2026-09-23T10:03:00Z")),
            ..AccountRuntime::default()
        },
    );
    let codex_home = usage_home_of(&work.reference);
    let claude_home = usage_home_of(&claude.reference);
    model.usage_homes = [codex_home.clone(), claude_home.clone()].into();
    model.usage.insert(codex_home, codex_usage());
    model.usage.insert(claude_home, claude_usage());
    model
        .settings
        .display
        .hidden_windows
        .insert("codex:work".into(), vec!["weekly".into()]);
    model
}

fn assemble_sample(model: &Model) -> StatePayload {
    let homes = HomeDisplay::new(Some(PathBuf::from("/home/ada")));
    let ctx = AssembleContext {
        now: ts(NOW),
        tz: &TimeZone::UTC,
        homes: &homes,
        catalog: &catalog(),
    };
    assemble(model, &ctx)
}

fn check_snapshot(name: &str, expected: &str, payload: &StatePayload) {
    assert_eq!(payload.app_version.as_deref(), Some(payload::APP_VERSION));
    let release_independent = StatePayload {
        app_version: Some(SNAPSHOT_APP_VERSION.to_owned()),
        ..payload.clone()
    };
    let actual = serde_json::to_string_pretty(&release_independent).unwrap() + "\n";
    if std::env::var_os("UPDATE_SNAPSHOTS").is_some() {
        let path = format!("{}/src/state/snapshots/{name}", env!("CARGO_MANIFEST_DIR"));
        std::fs::write(path, &actual).unwrap();
        return;
    }
    assert_eq!(actual, expected);
}

#[test]
fn full_state_matches_snapshot() {
    let payload = assemble_sample(&sample_model());
    check_snapshot(
        "state_full.json",
        include_str!("snapshots/state_full.json"),
        &payload,
    );
}

#[test]
fn empty_state_matches_snapshot() {
    let payload = assemble_sample(&Model::default());
    check_snapshot(
        "state_empty.json",
        include_str!("snapshots/state_empty.json"),
        &payload,
    );
}

#[test]
fn payload_parses_back_from_json() {
    let payload = assemble_sample(&sample_model());
    let json = serde_json::to_string(&payload).unwrap();
    let parsed: StatePayload = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.app_version, payload.app_version);
    assert_eq!(parsed.headline, payload.headline);
    assert_eq!(parsed.usage, payload.usage);
    assert_eq!(parsed.accounts.len(), payload.accounts.len());
    assert_eq!(parsed.accounts[0].balances, payload.accounts[0].balances);
}

#[test]
fn gone_accounts_are_left_out_and_hidden_are_flagged() {
    let payload = assemble_sample(&sample_model());
    let ids: Vec<_> = payload
        .accounts
        .iter()
        .map(|a| (a.id.as_str(), a.hidden))
        .collect();
    assert_eq!(
        ids,
        [
            ("codex:work", false),
            ("claude:main", false),
            ("codex:hidden", true)
        ]
    );
    assert_eq!(payload.accounts[1].status, AccountStatus::SignedOut);
    assert_eq!(payload.accounts[1].source, Some(DataSource::Cache));
    assert_eq!(payload.accounts[2].status, AccountStatus::Stale);
}

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
fn top_level_activity_and_spend_are_reported() {
    let payload = assemble_sample(&sample_model());
    assert_eq!(payload.next_refresh_at, Some(ts("2026-09-23T10:03:00Z")));
    assert_eq!(payload.last_success_at, Some(ts("2026-09-23T09:58:00Z")));
    assert!(!payload.offline);
    let today = &payload.spend.today;
    assert_eq!(today.cost_usd_micros, 12_400);
    assert_eq!(today.total_tokens, 6_200);
    let providers: Vec<_> = today
        .by_provider
        .iter()
        .map(|p| p.provider.clone())
        .collect();
    assert_eq!(providers, [CLAUDE, CODEX]);
    assert!(payload.spend.yesterday.partial);
}

#[test]
fn spare_is_reported_only_for_healthy_and_close_windows() {
    let payload = assemble_sample(&sample_model());
    let spare: Vec<_> = payload
        .accounts
        .iter()
        .flat_map(|a| a.windows.iter())
        .map(|w| (w.pace.severity, w.pace.spare_percent))
        .collect();
    let close = spare[0].1.unwrap();
    assert_eq!(spare[0].0, headroom_core::pace::Severity::Close);
    assert!(
        (close
            - (100.0
                - payload.accounts[0].windows[0]
                    .pace
                    .projected_percent
                    .unwrap()))
        .abs()
            < 1e-9
    );
    assert_eq!(spare[2].0, headroom_core::pace::Severity::RunningOut);
    assert_eq!(spare[2].1, None);
}

#[test]
fn accounts_report_their_credential_owner() {
    let mut model = sample_model();
    model.accounts[1].reference.owner = CredentialOwner::Headroom;
    let owners: Vec<_> = assemble_sample(&model)
        .accounts
        .iter()
        .map(|a| a.owner)
        .collect();
    assert_eq!(
        owners,
        [
            CredentialOwner::Cli,
            CredentialOwner::Headroom,
            CredentialOwner::Cli
        ]
    );
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
fn hidden_windows_are_flagged_in_the_payload() {
    let payload = assemble_sample(&sample_model());
    let flags: Vec<_> = payload.accounts[0]
        .windows
        .iter()
        .map(|w| (w.id.as_str(), w.hidden))
        .collect();
    assert_eq!(flags, [("session", false), ("weekly", true)]);
    assert!(payload.display.is_hidden("codex:work", "weekly"));
}

#[test]
fn translucent_display_setting_is_copied_into_the_payload() {
    let mut model = sample_model();
    assert!(!assemble_sample(&model).display.translucent);
    model.settings.display.translucent = true;
    let payload = assemble_sample(&model);
    assert!(payload.display.translucent);
    let json = serde_json::to_value(&payload).unwrap();
    assert_eq!(json["display"]["translucent"], serde_json::json!(true));
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
