use jiff::civil::date;

use super::*;

#[test]
fn periods_days_and_dates_are_parsed() {
    let table = [
        ("today", Since::Today),
        ("yesterday", Since::Yesterday),
        ("1d", Since::Days(1)),
        ("7d", Since::Days(7)),
        ("14d", Since::Days(14)),
        ("60d", Since::Days(60)),
        (" 30d ", Since::Days(30)),
        ("2026-09-01", Since::Date(date(2026, 9, 1))),
        ("99999999999999999999999d", Since::Days(u64::MAX)),
    ];
    for (text, expected) in table {
        assert_eq!(parse_since(text), Ok(expected), "{text}");
    }
}

#[test]
fn garbage_zero_days_and_impossible_dates_are_rejected() {
    for text in [
        "",
        "d",
        "abc",
        "-3d",
        "7 d",
        "7days",
        "2026-9-1",
        "2026/09/01",
    ] {
        assert_eq!(parse_since(text), Err(EXPECTED.to_owned()), "{text:?}");
    }
    assert!(parse_since("0d").unwrap_err().contains("at least 1"));
    let impossible = parse_since("2026-02-30").unwrap_err();
    assert!(impossible.contains("date is not valid"), "{impossible}");
}

fn query(since: Since) -> Value {
    let mut query = Map::new();
    since.insert_into(&mut query, date(2026, 9, 25));
    Value::Object(query)
}

#[test]
fn days_become_a_period_or_the_first_day_counting_today() {
    let table = [
        (Since::Today, r#"{"period":"today"}"#),
        (Since::Yesterday, r#"{"period":"yesterday"}"#),
        (Since::Days(7), r#"{"period":"7d"}"#),
        (Since::Days(30), r#"{"period":"30d"}"#),
        (Since::Days(1), r#"{"since":"2026-09-25"}"#),
        (Since::Days(14), r#"{"since":"2026-09-12"}"#),
        (Since::Days(60), r#"{"since":"2026-07-28"}"#),
        (Since::Date(date(2026, 9, 1)), r#"{"since":"2026-09-01"}"#),
    ];
    for (since, expected) in table {
        let expected: Value = serde_json::from_str(expected).unwrap();
        assert_eq!(query(since), expected, "{since:?}");
    }
}

#[test]
fn an_enormous_number_of_days_starts_at_the_earliest_date() {
    let expected = serde_json::json!({ "since": Date::MIN.to_string() });
    assert_eq!(query(Since::Days(u64::MAX)), expected);
}
