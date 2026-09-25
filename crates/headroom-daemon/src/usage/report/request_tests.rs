use super::*;
use crate::testing::{CODEX, catalog, ts};

const NOW: &str = "2026-09-23T10:00:00Z";

fn date(text: &str) -> Date {
    text.parse().unwrap()
}

fn resolved(json: &str) -> Result<SpendRequest, SpendQueryError> {
    resolve(json, &TimeZone::UTC, ts(NOW), &catalog())
}

fn dates_of(json: &str) -> (Date, Date) {
    let request = resolved(json).unwrap();
    (request.since, request.until)
}

#[test]
fn periods_end_today_in_the_local_zone() {
    let table = [
        ("today", "2026-09-23", "2026-09-23"),
        ("yesterday", "2026-09-22", "2026-09-22"),
        ("7d", "2026-09-17", "2026-09-23"),
        ("30d", "2026-08-25", "2026-09-23"),
    ];
    for (period, since, until) in table {
        let json = format!(r#"{{"period":"{period}","by":"model"}}"#);
        assert_eq!(dates_of(&json), (date(since), date(until)), "{period}");
    }
}

#[test]
fn since_defaults_until_to_today_and_ranges_follow_local_midnight() {
    let request = resolved(r#"{"since":"2026-09-20","by":"day"}"#).unwrap();
    assert_eq!(request.until, date("2026-09-23"));
    assert_eq!(request.by, GroupBy::Day);
    let almaty = TimeZone::get("Asia/Almaty").unwrap();
    let query = r#"{"since":"2026-09-20","until":"2026-09-21","by":"day"}"#;
    let local = resolve(query, &almaty, ts(NOW), &catalog()).unwrap();
    assert_eq!(local.range.since, ts("2026-09-19T19:00:00Z"));
    assert_eq!(local.range.until, ts("2026-09-21T19:00:00Z"));
}

#[test]
fn a_known_provider_filters_and_an_unknown_one_is_rejected() {
    let request = resolved(r#"{"period":"7d","by":"model","provider":"codex"}"#).unwrap();
    assert_eq!(request.provider, Some(CODEX));
    let unknown = resolved(r#"{"period":"7d","by":"model","provider":"grok"}"#);
    assert!(matches!(unknown, Err(SpendQueryError::UnknownProvider(id)) if id == "grok"));
}

#[test]
fn invalid_queries_are_typed_errors() {
    let table = [
        r#"{"by":"model"}"#,
        r#"{"period":"7d","since":"2026-09-20","by":"model"}"#,
        r#"{"period":"7d","until":"2026-09-20","by":"model"}"#,
        r#"{"until":"2026-09-20","by":"model"}"#,
        r#"{"since":"2026-09-21","until":"2026-09-20","by":"model"}"#,
        r#"{"since":"2026-09-24","by":"model"}"#,
        r#"{"since":"2026-09-20","until":"2026-09-24","by":"model"}"#,
        r#"{"since":"2026-08-20","by":"model"}"#,
        r#"{"period":"7d","by":"week"}"#,
        r#"{"period":"7d","by":"model","top":5}"#,
        r#"{"since":"20.09.2026","by":"model"}"#,
        "[]",
    ];
    for json in table {
        assert!(resolved(json).is_err(), "{json}");
    }
    let error = resolved(r#"{"since":"2026-08-20","by":"model"}"#).unwrap_err();
    assert_eq!(
        error.to_string(),
        "Headroom keeps usage for recent days only; the earliest since date is 2026-08-21"
    );
}

#[test]
fn the_earliest_since_date_stays_inside_retention() {
    assert_eq!(
        earliest_since(date("2026-09-23")).unwrap(),
        date("2026-08-21")
    );
    assert!(resolved(r#"{"since":"2026-08-21","by":"model"}"#).is_ok());
}
