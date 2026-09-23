use std::path::PathBuf;

use headroom_core::account::ProviderId;
use headroom_core::provider::ProviderError;
use jiff::tz::TimeZone;

use super::payload::{AccountStatus, AccountView};
use super::*;
use crate::home::HomeDisplay;
use crate::model::{AccountRuntime, RefreshFailure, SnapshotEntry, SnapshotOrigin};
use crate::storage::accounts::AccountRecord;
use crate::testing::{CLAUDE, CODEX, catalog};
use crate::testing::{account, session, snapshot, ts};

const NOW: &str = "2026-09-23T10:00:00Z";
const DETAIL: &str = "No active ChatGPT subscription (Free plan).";

fn record(provider: ProviderId, name: &str, order: i64) -> AccountRecord {
    AccountRecord {
        reference: account(provider, name),
        label: None,
        hidden: false,
        sort_order: order,
        email: Some("ada@example.com".into()),
        plan: Some("Plus".into()),
        last_seen: ts(NOW),
        gone: false,
    }
}

fn entry(used: f64) -> SnapshotEntry {
    SnapshotEntry {
        snapshot: snapshot(vec![session(used, "2026-09-23T12:00:00Z")], NOW),
        origin: SnapshotOrigin::Cache,
    }
}

fn lapsed_runtime() -> AccountRuntime {
    AccountRuntime {
        failure: Some(RefreshFailure::Provider(ProviderError::NoSubscription {
            detail: DETAIL.into(),
        })),
        failures: 1,
        ..AccountRuntime::default()
    }
}

fn model() -> Model {
    let mut work = record(CODEX, "work", 0);
    work.label = Some("Work".into());
    let claude = record(CLAUDE, "main", 1);
    let mut model = Model {
        accounts: vec![work.clone(), claude.clone()],
        ..Model::default()
    };
    model.snapshots.insert(work.id().clone(), entry(99.0));
    model.snapshots.insert(claude.id().clone(), entry(10.0));
    model.runtime.insert(work.id().clone(), lapsed_runtime());
    model
}

fn assemble_at_now(model: &Model) -> StatePayload {
    let homes = HomeDisplay::new(Some(PathBuf::from("/home/ada")));
    let ctx = AssembleContext {
        now: ts(NOW),
        tz: &TimeZone::UTC,
        homes: &homes,
        catalog: &catalog(),
    };
    assemble(model, &ctx)
}

fn lapsed_view(payload: &StatePayload) -> &AccountView {
    &payload.accounts[0]
}

#[test]
fn lapsed_account_matches_snapshot() {
    let payload = assemble_at_now(&model());
    let actual = serde_json::to_string_pretty(lapsed_view(&payload)).unwrap() + "\n";
    if std::env::var_os("UPDATE_SNAPSHOTS").is_some() {
        let dir = env!("CARGO_MANIFEST_DIR");
        let path = format!("{dir}/src/state/snapshots/account_no_subscription.json");
        std::fs::write(path, &actual).unwrap();
        return;
    }
    assert_eq!(
        actual,
        include_str!("snapshots/account_no_subscription.json")
    );
}

#[test]
fn lapsed_account_hides_its_last_snapshot() {
    let payload = assemble_at_now(&model());
    let view = lapsed_view(&payload);
    assert_eq!(view.status, AccountStatus::NoSubscription);
    let error = view.error.as_ref().unwrap();
    assert_eq!(
        (error.kind.as_str(), error.message.as_str()),
        ("no_subscription", DETAIL)
    );
    assert!(view.windows.is_empty() && view.balances.is_empty() && view.notices.is_empty());
    assert_eq!(
        (view.plan.as_ref(), view.updated_at, view.source),
        (None, None, None)
    );
}

#[test]
fn lapsed_account_never_becomes_the_headline() {
    let payload = assemble_at_now(&model());
    let headline = payload.headline.unwrap();
    assert_eq!(headline.account_id, "claude:main");
}

#[test]
fn refreshing_wins_over_no_subscription_but_data_stays_hidden() {
    let mut model = model();
    let id = model.accounts[0].id().clone();
    model.runtime_mut(&id).refreshing = true;
    let payload = assemble_at_now(&model);
    let view = lapsed_view(&payload);
    assert_eq!(view.status, AccountStatus::Refreshing);
    assert_eq!(view.error.as_ref().unwrap().kind, "no_subscription");
    assert!(view.windows.is_empty());
}
