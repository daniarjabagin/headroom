use super::*;
use crate::testing::ts;

fn clock(text: &str) -> ClockTime {
    ClockTime::try_from(text.to_owned()).unwrap()
}

fn hours(from: &str, to: &str) -> QuietHours {
    QuietHours {
        enabled: true,
        from: clock(from),
        to: clock(to),
        allow_critical: true,
    }
}

fn time(text: &str) -> Time {
    Time::strptime("%H:%M", text).unwrap()
}

fn berlin() -> TimeZone {
    TimeZone::get("Europe/Berlin").unwrap()
}

#[test]
fn ranges_include_the_start_and_exclude_the_end() {
    let night = hours("22:00", "08:00");
    let lunch = hours("13:00", "14:00");
    let cases = [
        (&night, "21:59", false),
        (&night, "22:00", true),
        (&night, "23:59", true),
        (&night, "00:00", true),
        (&night, "07:59", true),
        (&night, "08:00", false),
        (&night, "12:00", false),
        (&lunch, "12:59", false),
        (&lunch, "13:00", true),
        (&lunch, "13:59", true),
        (&lunch, "14:00", false),
        (&lunch, "23:00", false),
    ];
    for (range, at, expected) in cases {
        assert_eq!(is_quiet(time(at), *range), expected, "{at}");
    }
}

#[test]
fn disabled_or_empty_ranges_are_never_quiet() {
    let mut disabled = hours("22:00", "08:00");
    disabled.enabled = false;
    assert!(!is_quiet(time("23:00"), disabled));
    let empty = hours("22:00", "22:00");
    assert!(!is_quiet(time("22:00"), empty));
    assert!(!is_quiet(time("03:00"), empty));
    assert_eq!(
        quiet_ends(ts("2026-09-23T21:00:00Z"), &berlin(), disabled),
        None
    );
}

#[test]
fn quiet_follows_local_time_across_daylight_saving_days() {
    let night = hours("22:00", "08:00");
    let cases = [
        ("2026-03-28T20:59:00Z", false),
        ("2026-03-28T21:00:00Z", true),
        ("2026-03-29T01:30:00Z", true),
        ("2026-03-29T05:59:00Z", true),
        ("2026-03-29T06:00:00Z", false),
        ("2026-10-24T19:59:00Z", false),
        ("2026-10-24T20:00:00Z", true),
        ("2026-10-25T06:59:00Z", true),
        ("2026-10-25T07:00:00Z", false),
    ];
    for (at, expected) in cases {
        assert_eq!(is_quiet_at(ts(at), &berlin(), night), expected, "{at}");
    }
}

#[test]
fn quiet_ends_at_the_next_local_end_time() {
    let night = hours("22:00", "08:00");
    let lunch = hours("13:00", "14:00");
    let cases = [
        (&night, "2026-03-28T22:00:00Z", Some("2026-03-29T06:00:00Z")),
        (&night, "2026-03-29T03:00:00Z", Some("2026-03-29T06:00:00Z")),
        (&night, "2026-10-24T21:00:00Z", Some("2026-10-25T07:00:00Z")),
        (&night, "2026-09-23T12:00:00Z", None),
        (&lunch, "2026-09-23T11:30:00Z", Some("2026-09-23T12:00:00Z")),
        (&lunch, "2026-09-23T12:00:00Z", None),
    ];
    for (range, at, expected) in cases {
        let expected = expected.map(ts);
        assert_eq!(quiet_ends(ts(at), &berlin(), *range), expected, "{at}");
    }
}
