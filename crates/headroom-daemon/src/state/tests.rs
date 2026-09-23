use std::path::PathBuf;

use headroom_core::account::{AccountId, ProviderKind};
use headroom_core::pace::Tone;
use headroom_core::provider::ProviderError;
use headroom_core::quota::{Balance, BalanceAmount, LimitsSource, Notice};
use headroom_core::units::MicroUsd;
use headroom_core::usage::aggregate;

use super::payload::{AccountStatus, AccountView, DataSource, PaceView, WindowView};
use super::*;
use crate::home::UsageHome;
use crate::model::{AccountRuntime, RefreshFailure, SnapshotEntry, SnapshotOrigin};
use crate::settings::HeadlineMode;
use crate::storage::accounts::AccountRecord;
use crate::testing::{FlatPrices, account, event, session, snapshot, ts, weekly};

const NOW: &str = "2026-09-23T10:00:00Z";

fn record(provider: ProviderKind, name: &str, order: i64) -> AccountRecord {
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

fn sample_model() -> Model {
    let mut work = record(ProviderKind::Codex, "work", 0);
    work.label = Some("Work".into());
    let claude = record(ProviderKind::Claude, "main", 1);
    let mut hidden = record(ProviderKind::Codex, "hidden", 2);
    hidden.hidden = true;
    hidden.reference.home = PathBuf::from("/srv/codex");
    let mut gone = record(ProviderKind::Codex, "gone", 3);
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
    model
        .usage
        .insert(UsageHome::of(&work.reference), codex_usage());
    model
}

fn assemble_sample(model: &Model) -> StatePayload {
    let homes = HomeDisplay::new(Some(PathBuf::from("/home/ada")));
    let ctx = AssembleContext {
        now: ts(NOW),
        tz: &TimeZone::UTC,
        homes: &homes,
    };
    assemble(model, &ctx)
}

fn check_snapshot(name: &str, expected: &str, payload: &StatePayload) {
    let actual = serde_json::to_string_pretty(payload).unwrap() + "\n";
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

fn view(id: &str, hidden: bool, windows: Vec<WindowView>) -> AccountView {
    AccountView {
        id: id.into(),
        provider: ProviderKind::Codex,
        label: None,
        email: None,
        plan: None,
        hidden,
        status: AccountStatus::Fresh,
        error: None,
        updated_at: None,
        source: None,
        windows,
        balances: Vec::new(),
        notices: Vec::new(),
        usage_home: "~/.codex".into(),
    }
}

fn window(id: &str, remaining: f64, tone: Tone) -> WindowView {
    WindowView {
        id: id.into(),
        label: id.into(),
        used_percent: 100.0 - remaining,
        remaining_percent: remaining,
        resets_at: None,
        period_seconds: None,
        tone,
        pace: PaceView {
            severity: headroom_core::pace::Severity::Untracked,
            even_pace_percent: None,
            projected_percent: None,
            runs_out_at: None,
        },
    }
}

#[test]
fn headline_prefers_highest_tone_then_lowest_remaining() {
    let accounts = [
        view(
            "a",
            false,
            vec![
                window("session", 5.0, Tone::Good),
                window("weekly", 40.0, Tone::Warning),
            ],
        ),
        view("b", false, vec![window("session", 30.0, Tone::Warning)]),
        view("c", true, vec![window("session", 1.0, Tone::Critical)]),
    ];
    let chosen = headline::headline(&accounts, &HeadlineMode::Auto).unwrap();
    assert_eq!(
        (chosen.account_id.as_str(), chosen.window.as_str()),
        ("b", "session")
    );
    assert_eq!(chosen.tone, Tone::Warning);
    assert!((chosen.remaining_percent - 30.0).abs() < f64::EPSILON);
}

#[test]
fn headline_ties_keep_account_order() {
    let accounts = [
        view("a", false, vec![window("session", 50.0, Tone::Good)]),
        view("b", false, vec![window("session", 50.0, Tone::Good)]),
    ];
    let chosen = headline::headline(&accounts, &HeadlineMode::Auto).unwrap();
    assert_eq!(chosen.account_id, "a");
}

#[test]
fn pinned_headline_falls_back_to_auto_when_missing() {
    let accounts = [
        view("a", false, vec![window("session", 90.0, Tone::Good)]),
        view("b", false, vec![window("weekly", 10.0, Tone::Critical)]),
    ];
    let pin = |account: &str, window: &str| HeadlineMode::Pinned {
        account_id: account.into(),
        window: window.into(),
    };
    let chosen = headline::headline(&accounts, &pin("a", "session")).unwrap();
    assert_eq!(chosen.account_id, "a");
    let fallback = headline::headline(&accounts, &pin("a", "weekly")).unwrap();
    assert_eq!(fallback.account_id, "b");
    assert!(headline::headline(&[], &HeadlineMode::Auto).is_none());
}

#[test]
fn daily_trend_is_dense_over_thirty_days() {
    let payload = assemble_sample(&sample_model());
    let daily = &payload.usage[0].daily;
    assert_eq!(daily.len(), 30);
    assert_eq!(daily[0].date.to_string(), "2026-08-25");
    assert_eq!(daily[29].date.to_string(), "2026-09-23");
    assert_eq!(daily[29].total_tokens, 1_200);
    assert!(daily[28].partial);
    assert_eq!(daily[27].total_tokens, 0);
}

#[test]
fn usage_of_homes_without_active_accounts_is_omitted() {
    let mut model = sample_model();
    let stray = UsageHome {
        provider: ProviderKind::Claude,
        home: PathBuf::from("/nowhere"),
    };
    model.usage.insert(stray, codex_usage());
    model
        .accounts
        .retain(|a| a.id() != &AccountId("codex:work".into()));
    assert!(assemble_sample(&model).usage.is_empty());
}
