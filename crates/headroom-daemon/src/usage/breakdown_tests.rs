use std::path::PathBuf;

use headroom_core::cursor::LogCursors;
use headroom_core::tokens::TokenCounts;
use headroom_core::units::{MicroUsd, Tokens};

use super::*;
use crate::storage::Storage;
use crate::testing::{CLAUDE, CODEX, FlatPrices, event, ts};

const APP: &str = "/home/user/work/app";
const API: &str = "/home/user/work/api";

fn placed(key: &str, at: &str, model: &str, input: u64, project: Option<&str>) -> UsageEvent {
    UsageEvent {
        project: project.map(str::to_owned),
        ..event(key, at, model, input, 0)
    }
}

fn range(since: &str, until: &str) -> SpendRange {
    SpendRange {
        since: ts(since),
        until: ts(until),
    }
}

fn september(group_by: GroupBy, tz: &TimeZone) -> SpendQuery<'_> {
    SpendQuery {
        prices: &FlatPrices,
        tz,
        range: range("2026-09-01T00:00:00Z", "2026-10-01T00:00:00Z"),
        group_by,
    }
}

fn two_homes() -> Vec<HomeEvents> {
    vec![
        HomeEvents {
            provider: CODEX,
            events: vec![
                placed("c1", "2026-09-22T20:00:00Z", "gpt-5.5", 100, Some(APP)),
                placed("c2", "2026-09-23T09:00:00Z", "unknown", 40, Some(API)),
            ],
        },
        HomeEvents {
            provider: CLAUDE,
            events: vec![
                placed("a1", "2026-09-23T10:00:00Z", "claude-opus-5-5", 30, None),
                placed("a2", "2026-09-23T11:00:00Z", "gpt-5.5", 10, Some(APP)),
            ],
        },
    ]
}

fn rows(breakdown: &Breakdown) -> Vec<(GroupKey, u64, i64, bool)> {
    breakdown
        .rows
        .iter()
        .map(|row| {
            let t = &row.totals;
            (
                row.key.clone(),
                t.tokens.total().0,
                t.cost.0,
                t.is_partial(),
            )
        })
        .collect()
}

fn day(text: &str) -> GroupKey {
    GroupKey::Day(text.parse().unwrap())
}

#[test]
fn every_grouping_splits_the_same_total() {
    let utc = TimeZone::UTC;
    let project = |name: Option<&str>| GroupKey::Project(name.map(str::to_owned));
    let model =
        |provider: &ProviderId, name: &str| GroupKey::Model(provider.clone(), name.to_owned());
    let table = [
        (
            GroupBy::Model,
            vec![
                (model(&CODEX, "gpt-5.5"), 100, 200, false),
                (model(&CLAUDE, "claude-opus-5-5"), 30, 60, false),
                (model(&CLAUDE, "gpt-5.5"), 10, 20, false),
                (model(&CODEX, "unknown"), 40, 0, true),
            ],
        ),
        (
            GroupBy::Project,
            vec![
                (project(Some(APP)), 110, 220, false),
                (project(None), 30, 60, false),
                (project(Some(API)), 40, 0, true),
            ],
        ),
        (
            GroupBy::Provider,
            vec![
                (GroupKey::Provider(CODEX), 140, 200, true),
                (GroupKey::Provider(CLAUDE), 40, 80, false),
            ],
        ),
        (
            GroupBy::Day,
            vec![
                (day("2026-09-22"), 100, 200, false),
                (day("2026-09-23"), 80, 80, true),
            ],
        ),
    ];
    for (group_by, expected) in table {
        let result = breakdown(&two_homes(), &september(group_by, &utc));
        assert_eq!(rows(&result), expected, "{group_by:?}");
        assert_eq!(result.totals.tokens.total(), Tokens(180));
        assert_eq!(result.totals.cost, MicroUsd(280));
        assert_eq!(result.totals.unpriced_tokens, Tokens(40));
        assert!(result.totals.is_partial());
        let mut sum = UsageTotals::default();
        result.rows.iter().for_each(|row| sum.absorb(&row.totals));
        assert_eq!(sum, result.totals, "{group_by:?}");
    }
}

#[test]
fn days_follow_the_time_zone() {
    let table = [
        ("Asia/Almaty", vec![(day("2026-09-23"), 180, 280, true)]),
        (
            "America/Los_Angeles",
            vec![
                (day("2026-09-22"), 100, 200, false),
                (day("2026-09-23"), 80, 80, true),
            ],
        ),
    ];
    for (zone, expected) in table {
        let tz = TimeZone::get(zone).unwrap();
        let result = breakdown(&two_homes(), &september(GroupBy::Day, &tz));
        assert_eq!(rows(&result), expected, "{zone}");
    }
}

#[test]
fn the_range_is_half_open() {
    let utc = TimeZone::UTC;
    let spend = SpendQuery {
        range: range("2026-09-23T09:00:00Z", "2026-09-23T11:00:00Z"),
        ..september(GroupBy::Model, &utc)
    };
    let result = breakdown(&two_homes(), &spend);
    let keys: Vec<GroupKey> = result.rows.into_iter().map(|row| row.key).collect();
    assert_eq!(
        keys,
        [
            GroupKey::Model(CLAUDE, "claude-opus-5-5".into()),
            GroupKey::Model(CODEX, "unknown".into())
        ]
    );
}

#[test]
fn cache_tokens_are_counted_and_priced() {
    let cached = UsageEvent {
        tokens: TokenCounts {
            input: Tokens(5),
            cache_read: Tokens(1_000),
            cache_write_5m: Tokens(200),
            cache_write_1h: Tokens(30),
            output: Tokens(7),
            reasoning: Tokens(3),
        },
        ..event("k", "2026-09-23T09:00:00Z", "gpt-5.5", 0, 0)
    };
    let homes = [HomeEvents {
        provider: CODEX,
        events: vec![cached],
    }];
    let utc = TimeZone::UTC;
    let result = breakdown(&homes, &september(GroupBy::Model, &utc));
    assert_eq!(result.totals.tokens.cache_read, Tokens(1_000));
    assert_eq!(result.totals.tokens.cache_write(), Tokens(230));
    assert_eq!(result.totals.tokens.total(), Tokens(1_242));
    assert_eq!(result.totals.cost, MicroUsd(2_484));
    assert_eq!(result.totals.cost_per_mtok(), Some(MicroUsd(2_000_000)));
}

#[test]
fn local_days_start_and_end_at_local_midnight() {
    let table = [
        (
            "Asia/Almaty",
            "2026-09-23T10:00:00Z",
            7,
            "2026-09-16T19:00:00Z",
            "2026-09-23T19:00:00Z",
        ),
        (
            "America/Los_Angeles",
            "2026-09-23T03:00:00Z",
            1,
            "2026-09-22T07:00:00Z",
            "2026-09-23T07:00:00Z",
        ),
        (
            "Europe/Berlin",
            "2026-10-26T10:00:00Z",
            2,
            "2026-10-24T22:00:00Z",
            "2026-10-26T23:00:00Z",
        ),
        (
            "UTC",
            "2026-09-23T00:00:00Z",
            30,
            "2026-08-25T00:00:00Z",
            "2026-09-24T00:00:00Z",
        ),
    ];
    for (zone, now, days, since, until) in table {
        let tz = TimeZone::get(zone).unwrap();
        let days = NonZeroU32::new(days).unwrap();
        let spend = SpendRange::local_days(&tz, ts(now), days).unwrap();
        assert_eq!(spend, range(since, until), "{zone}");
    }
}

fn claude_home() -> UsageHome {
    UsageHome {
        provider: CLAUDE,
        home: PathBuf::from("/home/user/.claude"),
    }
}

#[test]
fn query_reads_deduplicated_rows_from_storage() {
    let storage = Storage::open_in_memory().unwrap();
    let home = claude_home();
    let chunks = [
        placed(
            "m1",
            "2026-09-23T09:00:00Z",
            "claude-opus-5-5",
            10,
            Some(APP),
        ),
        placed(
            "m1",
            "2026-09-23T09:00:00Z",
            "claude-opus-5-5",
            25,
            Some(APP),
        ),
        placed(
            "m1",
            "2026-09-23T09:00:00Z",
            "claude-opus-5-5",
            25,
            Some(APP),
        ),
        placed(
            "m2",
            "2026-08-01T09:00:00Z",
            "claude-opus-5-5",
            99,
            Some(APP),
        ),
    ];
    let utc = TimeZone::UTC;
    let result = storage
        .blocking(|conn| {
            for chunk in &chunks {
                let one = std::slice::from_ref(chunk);
                events::ingest(conn, &home, one, &LogCursors::default())?;
            }
            query(
                conn,
                &BTreeSet::from([home.clone()]),
                &september(GroupBy::Project, &utc),
            )
        })
        .unwrap();
    assert_eq!(
        rows(&result),
        [(GroupKey::Project(Some(APP.into())), 25, 50, false)]
    );
}
