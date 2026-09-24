use jiff::tz::TimeZone;

use super::*;
use crate::payload::{Pace, Tone};

fn at(text: &str) -> Timestamp {
    text.parse().unwrap()
}

fn utc(lang: Lang) -> Locale {
    Locale::new(lang, TimeZone::UTC)
}

fn window(severity: Severity, spare: Option<f64>, runs_out_at: Option<&str>) -> Window {
    Window {
        id: "session".into(),
        label: "Session".into(),
        used_percent: 38.0,
        remaining_percent: 62.0,
        resets_at: Some(at("2026-09-23T12:41:00Z")),
        tone: Tone::Good,
        pace: Pace {
            severity,
            even_pace_percent: Some(46.0),
            projected_percent: spare.map(|value| 100.0 - value),
            spare_percent: spare,
            runs_out_at: runs_out_at.map(at),
        },
        hidden: false,
    }
}

#[test]
fn percent_readings() {
    assert_eq!(percent_reading(Lang::En, 61.6, ValueMode::Left), "62% left");
    assert_eq!(percent_reading(Lang::En, -3.0, ValueMode::Used), "0% used");
    assert_eq!(
        percent_reading(Lang::Ru, 8.4, ValueMode::Left),
        "Осталось 8%"
    );
}

#[test]
fn durations() {
    let cases = [
        (30 * SECOND, false, "1m"),
        (5 * MINUTE, false, "5m"),
        (2 * HOUR + 5 * MINUTE, false, "2h 5m"),
        (4 * DAY + 5 * HOUR + 59 * MINUTE, false, "4d 5h"),
        (42 * SECOND, true, "42s"),
        (3 * MINUTE + 7 * SECOND, true, "3m 07s"),
        (2 * HOUR, true, "2h 0m"),
    ];
    for (ms, seconds, expected) in cases {
        assert_eq!(duration(Lang::En, ms, seconds), expected, "{ms}");
    }
    assert_eq!(
        duration(Lang::Ru, 2 * HOUR + 5 * MINUTE, false),
        "2 ч 5 мин"
    );
}

#[test]
fn reset_texts() {
    let now = at("2026-09-23T10:00:00Z");
    let locale = utc(Lang::En);
    let reset = Some(at("2026-09-23T12:41:00Z"));
    assert_eq!(
        reset_text(&locale, reset, now, ResetFormat::Countdown),
        "Resets in 2h 41m"
    );
    assert_eq!(
        reset_text(&locale, reset, now, ResetFormat::Exact),
        "Resets today at 12:41"
    );
    assert_eq!(
        reset_text(&locale, None, now, ResetFormat::Countdown),
        "Not started"
    );
    assert_eq!(
        reset_text(&locale, Some(now), now, ResetFormat::Countdown),
        "Reset pending"
    );
    assert_eq!(
        reset_text(&utc(Lang::Ru), reset, now, ResetFormat::Countdown),
        "Сброс через 2 ч 41 мин"
    );
}

#[test]
fn countdown_goes_live_in_the_last_hour() {
    let now = at("2026-09-23T10:00:00Z");
    assert!(is_countdown_live(Some(at("2026-09-23T10:30:00Z")), now));
    assert!(!is_countdown_live(Some(at("2026-09-23T11:30:00Z")), now));
    assert!(!is_countdown_live(Some(now), now));
    assert!(!is_countdown_live(None, now));
}

#[test]
fn forecasts() {
    let now = at("2026-09-23T10:00:00Z");
    let locale = utc(Lang::En);
    let display = Display::default();
    let healthy = window(Severity::Healthy, Some(33.4), None);
    assert_eq!(
        forecast_text(&locale, &healthy, now, &display).as_deref(),
        Some("At this pace: ~33% left at reset")
    );
    let used = Display {
        value_mode: ValueMode::Used,
        ..Display::default()
    };
    assert_eq!(
        forecast_text(&locale, &healthy, now, &used).as_deref(),
        Some("At this pace: ~67% used at reset")
    );
    let running = window(Severity::RunningOut, None, Some("2026-09-23T11:00:00Z"));
    assert_eq!(
        forecast_text(&locale, &running, now, &display).as_deref(),
        Some("At this pace: runs out in 1h 0m · resets in 2h 41m")
    );
    let late = window(Severity::RunningOut, None, Some("2026-09-23T09:00:00Z"));
    assert_eq!(
        forecast_text(&locale, &late, now, &display).as_deref(),
        Some("At this pace: runs out any minute")
    );
    let spent = window(Severity::Spent, None, None);
    assert_eq!(forecast_text(&locale, &spent, now, &display), None);
}

#[test]
fn pace_notes() {
    let now = at("2026-09-23T10:00:00Z");
    assert_eq!(spare_text(Lang::En, 8.3), "~8% spare");
    assert_eq!(
        limit_text(Lang::En, Some(at("2026-09-23T13:05:00Z")), now),
        "Limit in 3h 5m"
    );
    assert_eq!(limit_text(Lang::En, None, now), "Limit soon");
}

#[test]
fn footer_texts() {
    let now = at("2026-09-23T10:00:00Z");
    assert_eq!(
        next_update_text(Lang::En, at("2026-09-23T10:03:10Z"), now),
        "Next update in 3m"
    );
    assert_eq!(
        next_update_text(Lang::En, at("2026-09-23T10:00:20Z"), now),
        "Next update in <1m"
    );
    assert_eq!(
        ago_text(Lang::En, at("2026-09-23T07:00:00Z"), now),
        "3h 0m ago"
    );
    assert_eq!(ago_text(Lang::En, now, now), "just now");
    assert_eq!(
        updated_at_text(&utc(Lang::En), at("2026-09-23T09:58:00Z")),
        "Updated 09:58"
    );
}

#[test]
fn window_labels() {
    assert_eq!(window_label(Lang::Ru, "weekly", "Weekly"), "Неделя");
    assert_eq!(window_label(Lang::En, "model:opus", "Opus"), "Opus");
    assert_eq!(window_label(Lang::En, "other:x", ""), "other:x");
}
