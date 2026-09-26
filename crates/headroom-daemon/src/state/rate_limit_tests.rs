use std::path::PathBuf;

use headroom_core::provider::ProviderError;
use jiff::tz::TimeZone;

use super::payload::{AccountStatus, AccountView, RefreshReason};
use super::*;
use crate::home::HomeDisplay;
use crate::model::{AccountRuntime, RefreshFailure, SnapshotEntry, SnapshotOrigin};
use crate::storage::accounts::AccountRecord;
use crate::testing::{CLAUDE, account, catalog, session, snapshot, ts};

const NOW: &str = "2026-09-23T10:00:00Z";
const NEXT_TRY: &str = "2026-09-23T10:05:00Z";

fn claude() -> AccountRecord {
    AccountRecord {
        reference: account(CLAUDE, "main"),
        label: None,
        hidden: false,
        sort_order: 0,
        email: Some("ada@claude.example".into()),
        plan: Some("Max".into()),
        last_seen: ts(NOW),
        gone: false,
    }
}

fn limited_runtime() -> AccountRuntime {
    AccountRuntime {
        last_attempt: Some(ts("2026-09-23T09:55:00Z")),
        failure: Some(RefreshFailure::Provider(ProviderError::rate_limited(None))),
        failures: 2,
        rate_limits: 2,
        hold_until: Some(ts(NEXT_TRY)),
        next_refresh_at: Some(ts(NEXT_TRY)),
        ..AccountRuntime::default()
    }
}

fn model(fetched_at: Option<&str>) -> Model {
    let record = claude();
    let mut model = Model {
        accounts: vec![record.clone()],
        ..Model::default()
    };
    if let Some(fetched_at) = fetched_at {
        let limits = snapshot(vec![session(64.0, "2026-09-23T12:00:00Z")], fetched_at);
        let entry = SnapshotEntry {
            snapshot: limits,
            origin: SnapshotOrigin::Refreshed,
        };
        model.snapshots.insert(record.id().clone(), entry);
    }
    model.runtime.insert(record.id().clone(), limited_runtime());
    model
}

fn view_of(model: &Model) -> AccountView {
    let homes = HomeDisplay::new(Some(PathBuf::from("/home/ada")));
    let ctx = AssembleContext {
        now: ts(NOW),
        tz: &TimeZone::UTC,
        homes: &homes,
        catalog: &catalog(),
    };
    assemble(model, &ctx).accounts.remove(0)
}

#[test]
fn rate_limited_account_matches_snapshot() {
    let view = view_of(&model(Some("2026-09-23T09:40:00Z")));
    let actual = serde_json::to_string_pretty(&view).unwrap() + "\n";
    if std::env::var_os("UPDATE_SNAPSHOTS").is_some() {
        let dir = env!("CARGO_MANIFEST_DIR");
        let path = format!("{dir}/src/state/snapshots/account_rate_limited.json");
        std::fs::write(path, &actual).unwrap();
        return;
    }
    assert_eq!(actual, include_str!("snapshots/account_rate_limited.json"));
}

#[test]
fn a_rate_limit_keeps_the_last_windows_with_the_next_try() {
    let view = view_of(&model(Some("2026-09-23T09:40:00Z")));
    assert_eq!(view.status, AccountStatus::Stale);
    assert_eq!(view.windows.len(), 1);
    let error = view.error.as_ref().unwrap();
    assert_eq!(error.kind, "rate_limited");
    assert_eq!(error.message, "usage endpoint rate limited by the provider");
    assert_eq!(view.recovery, None);
    let refresh = view.refresh.unwrap();
    assert_eq!(refresh.next_at, Some(ts(NEXT_TRY)));
    assert_eq!(refresh.reason, RefreshReason::Hold);
}

#[test]
fn recent_data_stays_fresh_through_a_rate_limit() {
    let view = view_of(&model(Some("2026-09-23T09:55:00Z")));
    assert_eq!(view.status, AccountStatus::Fresh);
    assert_eq!(view.error.unwrap().kind, "rate_limited");
}

#[test]
fn a_rate_limit_without_data_is_an_error() {
    let view = view_of(&model(None));
    assert_eq!(view.status, AccountStatus::Error);
    assert!(view.windows.is_empty());
    assert_eq!(view.recovery, None);
}
