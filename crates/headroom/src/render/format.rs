use headroom_core::pace::{Severity, Tone};
use headroom_daemon::state::payload::{AccountView, PaceView, WindowView};
use jiff::{SignedDuration, Timestamp};

const MINUTE: i64 = 60;
const HOUR: i64 = 60 * MINUTE;
const DAY: i64 = 24 * HOUR;
const MICROS_PER_CENT: i64 = 10_000;
const TOKEN_UNITS: [(u64, &str); 3] = [(1_000_000_000, "B"), (1_000_000, "M"), (1_000, "K")];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Note {
    pub text: String,
    pub tone: Tone,
}

pub fn shown_windows(account: &AccountView) -> impl Iterator<Item = &WindowView> {
    account.windows.iter().filter(|window| !window.hidden)
}

pub fn account_title(account: &AccountView) -> String {
    let name = &account.provider_name;
    match account.label.as_deref().or(account.email.as_deref()) {
        Some(who) => format!("{name} · {who}"),
        None => name.clone(),
    }
}

pub fn rounded_percent(value: f64) -> String {
    format!("{:.0}", value.max(0.0).round())
}

pub fn percent_left(window: &WindowView) -> String {
    format!("{}% left", rounded_percent(window.remaining_percent))
}

pub fn duration(span: SignedDuration) -> String {
    let secs = span.as_secs().max(0);
    let (days, hours, minutes) = (secs / DAY, secs % DAY / HOUR, secs % HOUR / MINUTE);
    if days > 0 {
        format!("{days}d {hours}h")
    } else if hours > 0 {
        format!("{hours}h {minutes}m")
    } else {
        format!("{}m", minutes.max(1))
    }
}

pub fn reset_text(resets_at: Option<Timestamp>, now: Timestamp) -> String {
    let Some(resets_at) = resets_at else {
        return "not started".to_owned();
    };
    let left = resets_at.duration_since(now);
    if left.as_secs() < MINUTE {
        "resets soon".to_owned()
    } else {
        format!("resets in {}", duration(left))
    }
}

pub fn pace_note(pace: &PaceView, now: Timestamp) -> Option<Note> {
    let text = match pace.severity {
        Severity::Spent => "limit reached".to_owned(),
        Severity::RunningOut => limit_text(pace.runs_out_at, now),
        Severity::Close => format!("~{}% spare", rounded_percent(pace.spare_percent?)),
        Severity::Healthy | Severity::Untracked => return None,
    };
    let tone = match pace.severity {
        Severity::Close => Tone::Warning,
        _ => Tone::Critical,
    };
    Some(Note { text, tone })
}

fn limit_text(runs_out_at: Option<Timestamp>, now: Timestamp) -> String {
    match runs_out_at {
        Some(at) if at > now => format!("limit in {}", duration(at.duration_since(now))),
        _ => "limit soon".to_owned(),
    }
}

pub fn compact_tokens(count: u64) -> String {
    for (size, suffix) in TOKEN_UNITS {
        if count >= size {
            return format!("{}{suffix}", scaled(count, size));
        }
    }
    count.to_string()
}

fn scaled(count: u64, size: u64) -> String {
    let (count, size) = (u128::from(count), u128::from(size));
    if count / size >= 100 {
        return ((count + size / 2) / size).to_string();
    }
    let tenths = (count * 10 + size / 2) / size;
    if tenths % 10 == 0 {
        (tenths / 10).to_string()
    } else {
        format!("{}.{}", tenths / 10, tenths % 10)
    }
}

pub fn grouped(value: u64) -> String {
    let digits = value.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            out.push(',');
        }
        out.push(digit);
    }
    out
}

pub fn usd(micros: i64) -> String {
    let cents = rounded_cents(micros);
    let sign = if cents < 0 { "-" } else { "" };
    let cents = cents.unsigned_abs();
    format!("{sign}${}.{:02}", grouped(cents / 100), cents % 100)
}

fn rounded_cents(micros: i64) -> i64 {
    let half = MICROS_PER_CENT / 2;
    if micros >= 0 {
        micros.saturating_add(half) / MICROS_PER_CENT
    } else {
        micros.saturating_sub(half) / MICROS_PER_CENT
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ts(text: &str) -> Timestamp {
        text.parse().unwrap()
    }

    fn pace(severity: Severity, projected: Option<f64>, runs_out_at: Option<&str>) -> PaceView {
        let spare = matches!(severity, Severity::Healthy | Severity::Close)
            .then(|| projected.map(|value| 100.0 - value))
            .flatten();
        PaceView {
            severity,
            even_pace_percent: Some(50.0),
            projected_percent: projected,
            spare_percent: spare,
            runs_out_at: runs_out_at.map(ts),
        }
    }

    #[test]
    fn durations_use_the_two_largest_units() {
        assert_eq!(duration(SignedDuration::from_secs(9_660)), "2h 41m");
        assert_eq!(
            duration(SignedDuration::from_secs(4 * DAY + 6 * HOUR + 59)),
            "4d 6h"
        );
        assert_eq!(duration(SignedDuration::from_secs(30)), "1m");
        assert_eq!(duration(SignedDuration::from_secs(-30)), "1m");
    }

    #[test]
    fn reset_text_counts_down_from_now() {
        let now = ts("2026-09-23T10:00:00Z");
        assert_eq!(
            reset_text(Some(ts("2026-09-23T12:41:00Z")), now),
            "resets in 2h 41m"
        );
        assert_eq!(
            reset_text(Some(ts("2026-09-23T10:00:30Z")), now),
            "resets soon"
        );
        assert_eq!(reset_text(None, now), "not started");
    }

    #[test]
    fn pace_notes_follow_severity() {
        let now = ts("2026-09-23T10:00:00Z");
        let close = pace_note(&pace(Severity::Close, Some(95.6), None), now).unwrap();
        assert_eq!(
            close,
            Note {
                text: "~4% spare".into(),
                tone: Tone::Warning
            }
        );
        let running = pace(
            Severity::RunningOut,
            Some(130.0),
            Some("2026-09-24T19:00:00Z"),
        );
        assert_eq!(pace_note(&running, now).unwrap().text, "limit in 1d 9h");
        let past = pace(Severity::RunningOut, Some(130.0), None);
        assert_eq!(pace_note(&past, now).unwrap().text, "limit soon");
        let spent = pace_note(&pace(Severity::Spent, None, None), now).unwrap();
        assert_eq!(
            spent,
            Note {
                text: "limit reached".into(),
                tone: Tone::Critical
            }
        );
        assert_eq!(
            pace_note(&pace(Severity::Healthy, Some(60.0), None), now),
            None
        );
        assert_eq!(pace_note(&pace(Severity::Untracked, None, None), now), None);
    }

    #[test]
    fn spare_note_uses_the_daemon_spare_percent() {
        let now = ts("2026-09-23T10:00:00Z");
        let close = PaceView {
            spare_percent: Some(12.0),
            ..pace(Severity::Close, Some(95.0), None)
        };
        assert_eq!(pace_note(&close, now).unwrap().text, "~12% spare");
        let unknown = PaceView {
            spare_percent: None,
            ..pace(Severity::Close, Some(95.0), None)
        };
        assert_eq!(pace_note(&unknown, now), None);
    }

    #[test]
    fn percentages_round_half_away_from_zero() {
        assert_eq!(rounded_percent(61.5), "62");
        assert_eq!(rounded_percent(62.4), "62");
        assert_eq!(rounded_percent(-3.0), "0");
        assert_eq!(rounded_percent(150.0), "150");
    }

    #[test]
    fn tokens_are_abbreviated() {
        let cases = [
            (0, "0"),
            (999, "999"),
            (1_000, "1K"),
            (1_250, "1.3K"),
            (35_812_904, "35.8M"),
            (123_456_789, "123M"),
            (2_000_000_000, "2B"),
            (u64::MAX, "18446744074B"),
        ];
        for (count, expected) in cases {
            assert_eq!(compact_tokens(count), expected, "{count}");
        }
    }

    #[test]
    fn money_is_exact_to_the_cent() {
        assert_eq!(usd(0), "$0.00");
        assert_eq!(usd(2_400), "$0.00");
        assert_eq!(usd(5_000), "$0.01");
        assert_eq!(usd(12_500_000), "$12.50");
        assert_eq!(usd(1_234_567_890), "$1,234.57");
        assert_eq!(usd(-12_500_000), "-$12.50");
    }

    #[test]
    fn thousands_are_grouped() {
        assert_eq!(grouped(0), "0");
        assert_eq!(grouped(1_203_448), "1,203,448");
        assert_eq!(grouped(100), "100");
    }
}
