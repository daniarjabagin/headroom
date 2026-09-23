use headroom_core::quota::{QuotaWindow, WindowId};
use headroom_core::units::Percent;
use jiff::tz::TimeZone;
use jiff::{SignedDuration, Timestamp, ToSpan};

use super::client::{RawUsage, RawWindow};

const MONTHLY: &str = "monthly";

pub(super) fn map_usage(raw: &RawUsage) -> Vec<QuotaWindow> {
    let windows = &raw.usage;
    vec![
        window(
            WindowId::Session,
            "Session",
            &windows.rolling,
            Some(WindowId::SESSION_PERIOD),
        ),
        window(
            WindowId::Weekly,
            "Weekly",
            &windows.weekly,
            Some(WindowId::WEEKLY_PERIOD),
        ),
        window(
            WindowId::Other(MONTHLY.to_owned()),
            "Monthly",
            &windows.monthly,
            windows.monthly.resets_at.and_then(monthly_period),
        ),
    ]
}

fn window(
    id: WindowId,
    label: &str,
    raw: &RawWindow,
    period: Option<SignedDuration>,
) -> QuotaWindow {
    QuotaWindow {
        id,
        label: label.to_owned(),
        used: Percent::new(raw.percent),
        resets_at: raw.resets_at,
        period,
    }
}

fn monthly_period(resets_at: Timestamp) -> Option<SignedDuration> {
    let end = resets_at.to_zoned(TimeZone::UTC);
    let start = end.checked_sub(1.month()).ok()?;
    Some(resets_at.duration_since(start.timestamp()))
}

#[cfg(test)]
mod tests {
    use super::super::client::parse_usage;
    use super::*;

    const OK: &str = include_str!("fixtures/usage_ok.json");

    fn at(text: &str) -> Timestamp {
        text.parse().unwrap()
    }

    #[test]
    fn the_fixture_maps_to_session_weekly_and_monthly() {
        let windows = map_usage(&parse_usage(OK.as_bytes()).unwrap());
        let expected = [
            QuotaWindow {
                id: WindowId::Session,
                label: "Session".into(),
                used: Percent::new(12.0),
                resets_at: Some(at("2026-09-23T13:41:07.512Z")),
                period: Some(SignedDuration::from_hours(5)),
            },
            QuotaWindow {
                id: WindowId::Weekly,
                label: "Weekly".into(),
                used: Percent::new(37.0),
                resets_at: Some(at("2026-09-28T00:00:00.431Z")),
                period: Some(SignedDuration::from_hours(7 * 24)),
            },
            QuotaWindow {
                id: WindowId::Other("monthly".into()),
                label: "Monthly".into(),
                used: Percent::FULL,
                resets_at: Some(at("2026-10-14T08:15:30.004Z")),
                period: Some(SignedDuration::from_hours(30 * 24)),
            },
        ];
        assert_eq!(windows, expected);
    }

    #[test]
    fn windows_without_a_reset_keep_their_percent() {
        let body =
            r#"{"usage":{"rolling":{"percent":0},"weekly":{"percent":5},"monthly":{"percent":7}}}"#;
        let windows = map_usage(&parse_usage(body.as_bytes()).unwrap());
        let summary: Vec<_> = windows
            .iter()
            .map(|w| (w.used.value(), w.resets_at, w.period.is_some()))
            .collect();
        assert_eq!(
            summary,
            [(0.0, None, true), (5.0, None, true), (7.0, None, false)]
        );
    }

    #[test]
    fn the_monthly_period_is_the_calendar_month_before_the_reset() {
        let cases = [
            ("2026-10-14T08:15:30Z", 30),
            ("2026-03-31T00:00:00Z", 31),
            ("2026-03-01T12:00:00Z", 28),
            ("2028-03-01T12:00:00Z", 29),
            ("2027-01-05T00:00:00Z", 31),
        ];
        for (reset, days) in cases {
            assert_eq!(
                monthly_period(at(reset)),
                Some(SignedDuration::from_hours(days * 24)),
                "{reset}"
            );
        }
    }
}
