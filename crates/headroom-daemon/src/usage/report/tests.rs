use std::path::PathBuf;

use headroom_core::cursor::LogCursors;
use headroom_core::event::UsageEvent;
use serde_json::{Value, json};

use super::*;
use crate::clock::testing::ManualClock;
use crate::storage::events;
use crate::testing::{CLAUDE, CODEX, FlatPrices, catalog, event, ts};

const NOW: &str = "2026-09-23T10:00:00Z";

fn home(provider: &headroom_core::account::ProviderId, dir: PathBuf) -> UsageHome {
    UsageHome {
        provider: provider.clone(),
        home: dir,
    }
}

fn placed(key: &str, at: &str, model: &str, input: u64, project: Option<&str>) -> UsageEvent {
    UsageEvent {
        project: project.map(str::to_owned),
        ..event(key, at, model, input, 0)
    }
}

fn seed(storage: &Storage, codex: &UsageHome, claude: &UsageHome) {
    let codex_events = [
        placed(
            "c1",
            "2026-09-22T20:00:00Z",
            "gpt-5.5",
            100,
            Some("/home/ada/work/app"),
        ),
        placed("c2", "2026-09-23T09:00:00Z", "unknown", 40, None),
    ];
    let claude_events = [
        placed(
            "a1",
            "2026-09-21T10:00:00Z",
            "claude-opus",
            300,
            Some("/home/ada/work/app"),
        ),
        placed(
            "a2",
            "2026-09-23T08:00:00Z",
            "claude-opus",
            60,
            Some("/srv/api"),
        ),
    ];
    storage
        .blocking(|conn| {
            events::ingest(conn, codex, &codex_events, &LogCursors::default())?;
            events::ingest(conn, claude, &claude_events, &LogCursors::default())
        })
        .unwrap();
}

fn run(storage: &Storage, homes: &BTreeSet<UsageHome>, query: &str, tz: &TimeZone) -> Value {
    let request = request::resolve(query, tz, ts(NOW), &catalog()).unwrap();
    let display = HomeDisplay::new(Some("/home/ada".into()));
    let ctx = ReportContext {
        prices: &FlatPrices,
        tz,
        homes: &display,
    };
    let report = storage
        .blocking(|conn| build(conn, homes, &request, &ctx))
        .unwrap();
    serde_json::to_value(report).unwrap()
}

fn fixture() -> (Storage, BTreeSet<UsageHome>) {
    let storage = Storage::open_in_memory().unwrap();
    let codex = home(&CODEX, "/home/ada/.codex".into());
    let claude = home(&CLAUDE, "/home/ada/.claude".into());
    seed(&storage, &codex, &claude);
    (storage, BTreeSet::from([codex, claude]))
}

fn keys(report: &Value) -> Vec<(Value, Value, Value)> {
    report["rows"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| {
            (
                row["key"].clone(),
                row["cost_usd_micros"].clone(),
                row["share_permille"].clone(),
            )
        })
        .collect()
}

#[test]
fn models_carry_their_provider_and_the_total_adds_up() {
    let (storage, homes) = fixture();
    let report = run(
        &storage,
        &homes,
        r#"{"period":"7d","by":"model"}"#,
        &TimeZone::UTC,
    );
    assert_eq!(report["since"], "2026-09-17");
    assert_eq!(report["until"], "2026-09-23");
    assert_eq!(report["by"], "model");
    assert_eq!(
        keys(&report),
        [
            (json!("claude-opus"), json!(720), json!(782)),
            (json!("gpt-5.5"), json!(200), json!(217)),
            (json!("unknown"), json!(0), json!(0)),
        ]
    );
    let providers: Vec<&Value> = report["rows"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| &row["provider"])
        .collect();
    assert_eq!(
        providers,
        [&json!("claude"), &json!("codex"), &json!("codex")]
    );
    let total = &report["total"];
    assert_eq!(total["key"], Value::Null);
    assert_eq!(total["provider"], Value::Null);
    assert_eq!(total["tokens"]["total"], 500);
    assert_eq!(total["cost_usd_micros"], 920);
    assert_eq!(total["unpriced_tokens"], 40);
    assert_eq!(total["partial"], true);
    assert_eq!(total["cost_per_mtok_usd_micros"], 2_000_000);
    assert_eq!(total["share_permille"], 1_000);
    assert_eq!(report["rows"][2]["cost_per_mtok_usd_micros"], Value::Null);
}

#[test]
fn projects_are_home_relative_with_null_for_no_directory() {
    let (storage, homes) = fixture();
    let report = run(
        &storage,
        &homes,
        r#"{"period":"7d","by":"project"}"#,
        &TimeZone::UTC,
    );
    assert_eq!(
        keys(&report),
        [
            (json!("~/work/app"), json!(800), json!(869)),
            (json!("/srv/api"), json!(120), json!(130)),
            (Value::Null, json!(0), json!(0)),
        ]
    );
    assert!(report["rows"][0]["provider"].is_null());
}

#[test]
fn days_are_local_and_include_days_without_usage() {
    let (storage, homes) = fixture();
    let almaty = TimeZone::get("Asia/Almaty").unwrap();
    let query = r#"{"since":"2026-09-20","by":"day"}"#;
    let report = run(&storage, &homes, query, &almaty);
    assert_eq!(
        keys(&report),
        [
            (json!("2026-09-20"), json!(0), json!(0)),
            (json!("2026-09-21"), json!(600), json!(652)),
            (json!("2026-09-22"), json!(0), json!(0)),
            (json!("2026-09-23"), json!(320), json!(347)),
        ]
    );
    let utc = run(&storage, &homes, query, &TimeZone::UTC);
    assert_eq!(utc["rows"][2]["key"], "2026-09-22");
    assert_eq!(utc["rows"][2]["cost_usd_micros"], 200);
}

#[test]
fn a_provider_filter_keeps_only_its_homes() {
    let (storage, homes) = fixture();
    let query = r#"{"period":"30d","by":"provider","provider":"codex"}"#;
    let report = run(&storage, &homes, query, &TimeZone::UTC);
    assert_eq!(keys(&report), [(json!("codex"), json!(200), json!(1_000))]);
    assert_eq!(report["rows"][0]["provider"], "codex");
    assert_eq!(report["total"]["tokens"]["total"], 140);
}

#[test]
fn the_report_round_trips_through_json() {
    let (storage, homes) = fixture();
    let report = run(
        &storage,
        &homes,
        r#"{"period":"7d","by":"model"}"#,
        &TimeZone::UTC,
    );
    let parsed: rows::SpendReport = serde_json::from_value(report.clone()).unwrap();
    assert_eq!(serde_json::to_value(parsed).unwrap(), report);
}

#[test]
fn once_reads_existing_homes_from_the_database() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("headroom.db");
    let codex = home(&CODEX, dir.path().join("codex"));
    let claude = home(&CLAUDE, dir.path().join("gone"));
    std::fs::create_dir_all(&codex.home).unwrap();
    seed(&Storage::open(&path).unwrap(), &codex, &claude);
    let clock = ManualClock::at(NOW);
    let display = HomeDisplay::default();
    let ctx = OnceSpendContext {
        price_book: &FlatPrices,
        clock: &clock,
        tz: &TimeZone::UTC,
        homes: &display,
        catalog: &catalog(),
    };
    let report = spend_report_once(&path, r#"{"period":"30d","by":"provider"}"#, &ctx).unwrap();
    assert_eq!(report.rows.len(), 1);
    assert_eq!(report.total.tokens.total, 140);
    let invalid = spend_report_once(&path, r#"{"by":"provider"}"#, &ctx).unwrap_err();
    assert!(invalid.is_invalid_argument());
}
