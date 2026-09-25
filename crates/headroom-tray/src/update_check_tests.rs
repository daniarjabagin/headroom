use jiff::tz::TimeZone;

use super::*;
use crate::i18n::Lang;

const NOW: &str = "2026-09-23T10:05:00Z";

fn at(text: &str) -> Timestamp {
    text.parse().unwrap()
}

fn line(lang: Lang, checked_at: Option<&str>, run: &CheckRun) -> String {
    let locale = Locale::new(lang, TimeZone::UTC);
    let ctx = CheckContext {
        locale: &locale,
        version: "0.6.0",
        checked_at: checked_at.map(at),
        now: at(NOW),
    };
    check_line(&ctx, run)
}

fn done(json: &str) -> CheckRun {
    CheckRun::Done(parse_outcome(json).unwrap())
}

#[test]
fn parses_every_documented_outcome() {
    let available = parse_outcome(
        r#"{"status":"available","checked_at":"2026-09-23T10:00:00Z","version":"0.6.1"}"#,
    )
    .unwrap();
    assert_eq!(available.status, CheckStatus::Available);
    assert_eq!(available.version.as_deref(), Some("0.6.1"));
    let limited = parse_outcome(
        r#"{"status":"rate_limited","checked_at":null,"version":null,"until":"2026-09-23T11:30:00Z"}"#,
    )
    .unwrap();
    assert_eq!(limited.until, Some(at("2026-09-23T11:30:00Z")));
    let future = parse_outcome(r#"{"status":"paused","extra":1}"#).unwrap();
    assert_eq!(future.status, CheckStatus::Unknown);
    assert_eq!(future.checked_at, None);
    assert!(parse_outcome(r#"{"checked_at":null}"#).is_err());
}

#[test]
fn the_row_needs_checks_on_a_known_check_and_a_new_daemon() {
    let check = UpdateCheck { checked_at: None };
    assert!(check_row_visible(Some("0.6.0"), Some(&check), true));
    assert!(check_row_visible(Some("v0.7.2-rc.1"), Some(&check), true));
    assert!(check_row_visible(Some("1.0.0"), Some(&check), true));
    assert!(!check_row_visible(Some("0.5.9"), Some(&check), true));
    assert!(!check_row_visible(Some("0.6"), Some(&check), true));
    assert!(!check_row_visible(None, Some(&check), true));
    assert!(!check_row_visible(Some("0.6.0"), None, true));
    assert!(!check_row_visible(Some("0.6.0"), Some(&check), false));
}

#[test]
fn idle_lines_show_the_last_check() {
    let checked = Some("2026-09-23T10:04:30Z");
    assert_eq!(
        line(Lang::En, checked, &CheckRun::Idle),
        "You're up to date · Headroom 0.6.0 · checked just now"
    );
    assert_eq!(
        line(Lang::En, Some("2026-09-23T09:00:00Z"), &CheckRun::Idle),
        "You're up to date · Headroom 0.6.0 · checked 1h 5m ago"
    );
    assert_eq!(
        line(Lang::En, None, &CheckRun::Idle),
        "Headroom 0.6.0 · not checked yet"
    );
    assert_eq!(
        line(Lang::Ru, checked, &CheckRun::Idle),
        "У вас последняя версия · Headroom 0.6.0 · проверено только что"
    );
    assert_eq!(
        line(Lang::En, checked, &CheckRun::Checking),
        "Checking for updates…"
    );
}

#[test]
fn outcomes_label_the_row() {
    let checked = Some("2026-09-23T10:00:00Z");
    let cases = [
        (
            r#"{"status":"up_to_date","checked_at":"2026-09-23T10:00:00Z","version":"0.6.0"}"#,
            "You're up to date · Headroom 0.6.0 · checked 5m ago",
        ),
        (
            r#"{"status":"available","checked_at":"2026-09-23T10:00:00Z","version":"0.6.1"}"#,
            "Headroom 0.6.1 is available · checked 5m ago",
        ),
        (
            r#"{"status":"failed","checked_at":"2026-09-23T10:00:00Z","version":null}"#,
            "Couldn't check for updates · last checked 5m ago",
        ),
        (
            r#"{"status":"rate_limited","checked_at":"2026-09-23T10:00:00Z","version":null,"until":"2026-09-23T11:30:00Z"}"#,
            "GitHub is limiting checks until today at 11:30",
        ),
        (
            r#"{"status":"rate_limited","checked_at":"2026-09-23T10:00:00Z","version":null,"until":"2026-09-23T10:00:00Z"}"#,
            "GitHub is limiting checks. Try again later.",
        ),
    ];
    for (json, expected) in cases {
        assert_eq!(line(Lang::En, checked, &done(json)), expected, "{json}");
    }
    let disabled = done(r#"{"status":"disabled","checked_at":null,"version":null}"#);
    assert_eq!(line(Lang::En, None, &disabled), "Update checks are off");
    let never = CheckRun::Done(CheckOutcome::failed(None));
    assert_eq!(line(Lang::En, None, &never), "Couldn't check for updates");
}

#[test]
fn a_newer_scheduled_check_replaces_an_old_failure() {
    let failed = done(r#"{"status":"failed","checked_at":"2026-09-23T08:00:00Z","version":null}"#);
    assert_eq!(
        line(Lang::En, Some("2026-09-23T10:04:00Z"), &failed),
        "You're up to date · Headroom 0.6.0 · checked 1m ago"
    );
    assert_eq!(
        line(Lang::Ru, Some("2026-09-23T08:00:00Z"), &failed),
        "Не удалось проверить обновления · последняя проверка 2 ч 5 мин назад"
    );
}
