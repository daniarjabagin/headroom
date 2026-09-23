use super::*;

const TOKENS: &str = include_str!("fixtures/quota_tokens.json");
const CREDITS: &str = include_str!("fixtures/quota_credits.json");
const NO_PLAN: &str = include_str!("fixtures/no_plan.json");
const SUBSCRIPTION: &str = include_str!("fixtures/subscription.json");

fn at(text: &str) -> Timestamp {
    text.parse().unwrap()
}

fn mapped(body: &str) -> Vec<QuotaWindow> {
    windows(&quota_limits(body.as_bytes()).unwrap()).unwrap()
}

fn limit(kind: &str, unit: i64, number: i64) -> RawLimit {
    RawLimit {
        kind: Some(kind.into()),
        unit: Some(unit),
        number: Some(number),
        percentage: Some(40.0),
        ..RawLimit::default()
    }
}

#[test]
fn legacy_token_limits_map_to_session_weekly_and_web_searches() {
    assert_eq!(
        mapped(TOKENS),
        [
            QuotaWindow {
                id: WindowId::Session,
                label: "Session".into(),
                used: Percent::new(17.0),
                resets_at: Some(at("2026-09-23T13:00:00Z")),
                period: Some(WindowId::SESSION_PERIOD),
            },
            QuotaWindow {
                id: WindowId::Weekly,
                label: "Weekly".into(),
                used: Percent::new(3.0),
                resets_at: Some(at("2026-09-29T00:00:00Z")),
                period: Some(WindowId::WEEKLY_PERIOD),
            },
            QuotaWindow {
                id: WindowId::Other("web_search".into()),
                label: "Web searches".into(),
                used: Percent::new(12.5),
                resets_at: Some(at("2026-10-13T00:00:00Z")),
                period: Some(SignedDuration::from_hours(30 * 24)),
            },
        ]
    );
}

#[test]
fn credit_limits_use_the_exact_counts_over_the_rounded_percentage() {
    let windows = mapped(CREDITS);
    assert_eq!(windows.len(), 2);
    assert_eq!(windows[0].id, WindowId::Session);
    assert_eq!(windows[0].used, Percent::ZERO);
    assert_eq!(windows[0].resets_at, None);
    assert_eq!(windows[1].id, WindowId::Weekly);
    assert_eq!(windows[1].used, Percent::new(98.55));
}

#[test]
fn no_coding_plan_is_a_missing_subscription() {
    assert_eq!(
        quota_limits(NO_PLAN.as_bytes()),
        Err(ProviderError::NoSubscription {
            detail: "No active GLM Coding Plan on this Z.ai account.".into()
        })
    );
}

#[test]
fn other_refusals_and_shapeless_bodies_are_invalid() {
    let refused = br#"{"code":500,"msg":"internal error","success":false}"#;
    assert_eq!(
        quota_limits(refused),
        Err(ProviderError::InvalidResponse(
            "Z.ai refused the quota request (code 500)".into()
        ))
    );
    assert!(quota_limits(br#"{"code":200,"success":true,"data":{}}"#).is_err());
    assert!(quota_limits(b"<html>").is_err());
}

#[test]
fn limits_may_sit_at_the_root_and_may_be_empty() {
    let root = br#"{"limits":[{"type":"TOKENS_LIMIT","unit":3,"number":5,"percentage":1}]}"#;
    assert_eq!(quota_limits(root).unwrap().len(), 1);
    let empty = br#"{"success":true,"data":{"limits":[]}}"#;
    assert_eq!(windows(&quota_limits(empty).unwrap()), Ok(Vec::new()));
}

#[test]
fn window_lengths_follow_the_unit_codes() {
    let cases = [
        (3, 5, WindowId::Session, "Session"),
        (6, 1, WindowId::Weekly, "Weekly"),
        (4, 7, WindowId::Weekly, "Weekly"),
        (4, 1, WindowId::Other("1d".into()), "1-day"),
        (3, 1, WindowId::Other("1h".into()), "1-hour"),
        (5, 1, WindowId::Other("monthly".into()), "Monthly"),
    ];
    for (unit, number, id, label) in cases {
        let window = percent_window(&limit("CREDIT_LIMIT", unit, number))
            .unwrap()
            .unwrap();
        assert_eq!(
            (window.id, window.label.as_str()),
            (id, label),
            "{unit}x{number}"
        );
    }
}

#[test]
fn unknown_units_are_skipped_and_broken_windows_are_errors() {
    assert_eq!(percent_window(&limit("CREDIT_LIMIT", 9, 1)), Ok(None));
    assert!(percent_window(&limit("CREDIT_LIMIT", 3, 0)).is_err());
    assert!(percent_window(&limit("CREDIT_LIMIT", 3, i64::MAX)).is_err());
    let lengthless = RawLimit {
        unit: None,
        ..limit("CREDIT_LIMIT", 3, 5)
    };
    assert!(percent_window(&lengthless).is_err());
    let usageless = RawLimit {
        percentage: None,
        ..limit("CREDIT_LIMIT", 3, 5)
    };
    assert!(percent_window(&usageless).is_err());
}

#[test]
fn names_stand_in_for_types_and_duplicates_keep_the_first() {
    let by_name = RawLimit {
        kind: None,
        name: Some("TOKENS_LIMIT".into()),
        ..limit("", 3, 5)
    };
    let second = RawLimit {
        percentage: Some(90.0),
        ..limit("CREDIT_LIMIT", 3, 5)
    };
    let other = limit("CONCURRENCY_LIMIT", 3, 5);
    let windows = windows(&[by_name, second, other]).unwrap();
    assert_eq!(windows.len(), 1);
    assert_eq!(windows[0].used, Percent::new(40.0));
}

#[test]
fn the_plan_name_comes_from_the_first_subscription() {
    assert_eq!(
        plan(SUBSCRIPTION.as_bytes()).as_deref(),
        Some("GLM Coding Pro")
    );
    assert_eq!(plan(br#"{"data":[]}"#), None);
    assert_eq!(plan(br#"{"data":[{"productName":"  "}]}"#), None);
    assert_eq!(plan(b"nope"), None);
}
