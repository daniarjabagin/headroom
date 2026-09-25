use headroom_core::account::ProviderId;
use headroom_core::cursor::LogCursors;
use headroom_core::event::{EventKey, ServiceTier, UsageEvent};
use headroom_core::tokens::TokenCounts;
use headroom_core::units::Tokens;
use headroom_daemon::home::UsageHome;
use headroom_daemon::storage::{Storage, events};
use jiff::{SignedDuration, Timestamp};

use super::*;

fn args(since: &str, until: Option<&str>, by: SpendBy, provider: Option<&str>) -> SpendArgs {
    SpendArgs {
        by,
        since: parse_since(since).unwrap(),
        until: until.map(|day| day.parse().unwrap()),
        provider: provider.map(str::to_owned),
        json: false,
    }
}

#[test]
fn periods_and_dates_become_the_get_spend_query() {
    let table = [
        (
            args("7d", None, SpendBy::Model, None),
            r#"{"by":"model","period":"7d"}"#,
        ),
        (
            args("30d", None, SpendBy::Project, Some("claude")),
            r#"{"by":"project","period":"30d","provider":"claude"}"#,
        ),
        (
            args("2026-09-01", Some("2026-09-15"), SpendBy::Day, None),
            r#"{"by":"day","since":"2026-09-01","until":"2026-09-15"}"#,
        ),
        (
            args("yesterday", None, SpendBy::Provider, None),
            r#"{"by":"provider","period":"yesterday"}"#,
        ),
        (
            args("60d", None, SpendBy::Model, None),
            r#"{"by":"model","since":"2026-07-28"}"#,
        ),
    ];
    for (args, expected) in table {
        let today = jiff::civil::date(2026, 9, 25);
        let query: Value = serde_json::from_str(&query(&args, today)).unwrap();
        let expected: Value = serde_json::from_str(expected).unwrap();
        assert_eq!(query, expected);
    }
}

fn recent_event(dir: &str) -> UsageEvent {
    UsageEvent {
        key: EventKey("r1".into()),
        at: Timestamp::now() - SignedDuration::from_mins(1),
        model: "no-such-model".into(),
        tier: ServiceTier::Standard,
        tokens: TokenCounts {
            input: Tokens(1_200),
            output: Tokens(300),
            ..TokenCounts::default()
        },
        web_search_requests: 0,
        reported_cost: None,
        project: Some(dir.into()),
    }
}

#[test]
fn without_a_daemon_the_database_is_read_directly() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("headroom.db");
    let home = UsageHome {
        provider: ProviderId::from_static("codex"),
        home: dir.path().join("codex"),
    };
    std::fs::create_dir_all(&home.home).unwrap();
    let project = dir.path().join("app");
    let event = recent_event(&project.to_string_lossy());
    Storage::open(&db)
        .unwrap()
        .blocking(|conn| events::ingest(conn, &home, &[event], &LogCursors::default()))
        .unwrap();
    let report = read_database(&db, dir.path(), r#"{"period":"7d","by":"project"}"#).unwrap();
    assert_eq!(report.rows.len(), 1);
    assert_eq!(report.rows[0].tokens.total, 1_500);
    assert_eq!(report.rows[0].unpriced_tokens, 1_500);
    assert!(report.total.partial);
    let invalid = read_database(&db, dir.path(), r#"{"by":"project"}"#).unwrap_err();
    assert!(
        invalid
            .to_string()
            .contains("needs either a period or a since date")
    );
}

#[test]
fn days_beyond_retention_get_the_daemon_retention_error() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("headroom.db");
    Storage::open(&db).unwrap();
    let today = TimeZone::system().to_datetime(Timestamp::now()).date();
    for since in ["60d", "99999999999999999999d"] {
        let request = query(&args(since, None, SpendBy::Model, None), today);
        let error = read_database(&db, dir.path(), &request).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("keeps usage for recent days only"),
            "{since}: {error:#}"
        );
    }
    let request = query(&args("14d", None, SpendBy::Model, None), today);
    assert!(
        read_database(&db, dir.path(), &request)
            .unwrap()
            .rows
            .is_empty()
    );
}

#[test]
fn a_missing_database_is_explained() {
    let dir = tempfile::tempdir().unwrap();
    let error = read_database(&dir.path().join("none.db"), dir.path(), "{}").unwrap_err();
    assert!(error.to_string().contains("no usage data yet"));
}
