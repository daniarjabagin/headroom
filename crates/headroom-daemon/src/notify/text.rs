use headroom_core::account::ProviderKind;
use headroom_core::pace::Severity;
use jiff::{SignedDuration, Timestamp};

use super::evaluator::{Milestone, Observation};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Notification {
    pub title: String,
    pub body: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Subject<'a> {
    pub provider: ProviderKind,
    pub account_name: Option<&'a str>,
    pub window_label: &'a str,
}

#[must_use]
pub fn compose(
    milestone: Milestone,
    subject: &Subject<'_>,
    observed: &Observation,
    now: Timestamp,
) -> Notification {
    Notification {
        title: title(subject),
        body: body(milestone, observed, now),
    }
}

fn title(subject: &Subject<'_>) -> String {
    let provider = provider_name(subject.provider);
    match subject.account_name {
        Some(name) => format!("{provider} · {name} — {}", subject.window_label),
        None => format!("{provider} — {}", subject.window_label),
    }
}

fn provider_name(kind: ProviderKind) -> String {
    let name = kind.as_str();
    let mut chars = name.chars();
    chars.next().map_or_else(String::new, |first| {
        first.to_uppercase().chain(chars).collect()
    })
}

fn body(milestone: Milestone, observed: &Observation, now: Timestamp) -> String {
    let headline = match milestone {
        Milestone::AlmostOut => "Under 10% left".to_owned(),
        Milestone::CuttingItClose => "Projected to finish close to the limit".to_owned(),
        Milestone::WillRunOut => running_out(observed, now),
        Milestone::Reset => format!("Limit reset · {}% left", whole_percent(observed.remaining)),
    };
    match resets_in(observed, now) {
        Some(countdown) if milestone != Milestone::Reset => {
            format!("{headline} · resets in {countdown}")
        }
        _ => headline,
    }
}

fn running_out(observed: &Observation, now: Timestamp) -> String {
    if observed.severity == Severity::Spent {
        return "Limit reached".to_owned();
    }
    match observed.runs_out_at {
        Some(at) => format!(
            "Projected to run out in {}",
            countdown(at.duration_since(now))
        ),
        None => "Projected to run out before the reset".to_owned(),
    }
}

fn resets_in(observed: &Observation, now: Timestamp) -> Option<String> {
    observed
        .resets_at
        .map(|at| at.duration_since(now))
        .filter(SignedDuration::is_positive)
        .map(countdown)
}

fn whole_percent(value: f64) -> String {
    format!("{:.0}", value.floor().clamp(0.0, 100.0))
}

#[must_use]
pub fn countdown(duration: SignedDuration) -> String {
    let minutes = duration.as_secs().max(0) / 60;
    let (days, hours, mins) = (minutes / 1_440, minutes / 60 % 24, minutes % 60);
    match (days, hours, mins) {
        (0, 0, 0) => "<1m".to_owned(),
        (0, 0, m) => format!("{m}m"),
        (0, h, 0) => format!("{h}h"),
        (0, h, m) => format!("{h}h {m}m"),
        (d, 0, _) => format!("{d}d"),
        (d, h, _) => format!("{d}d {h}h"),
    }
}

#[cfg(test)]
mod tests {
    use headroom_core::pace::Tone;

    use super::*;
    use crate::testing::ts;

    const NOW: &str = "2026-09-23T10:00:00Z";

    fn observed(severity: Severity, remaining: f64) -> Observation {
        Observation {
            remaining,
            severity,
            tone: Tone::Critical,
            resets_at: Some(ts("2026-09-23T10:42:00Z")),
            runs_out_at: Some(ts("2026-09-23T10:20:00Z")),
        }
    }

    fn work<'a>() -> Subject<'a> {
        Subject {
            provider: ProviderKind::Codex,
            account_name: Some("Work"),
            window_label: "Session",
        }
    }

    #[test]
    fn almost_out_reads_naturally() {
        let text = compose(
            Milestone::AlmostOut,
            &work(),
            &observed(Severity::Close, 8.0),
            ts(NOW),
        );
        assert_eq!(text.title, "Codex · Work — Session");
        assert_eq!(text.body, "Under 10% left · resets in 42m");
    }

    #[test]
    fn running_out_mentions_the_projection() {
        let at = ts(NOW);
        let text = compose(
            Milestone::WillRunOut,
            &work(),
            &observed(Severity::RunningOut, 30.0),
            at,
        );
        assert_eq!(text.body, "Projected to run out in 20m · resets in 42m");
        let spent = compose(
            Milestone::WillRunOut,
            &work(),
            &observed(Severity::Spent, 0.0),
            at,
        );
        assert_eq!(spent.body, "Limit reached · resets in 42m");
    }

    #[test]
    fn reset_and_unnamed_accounts() {
        let subject = Subject {
            provider: ProviderKind::Claude,
            account_name: None,
            window_label: "Weekly",
        };
        let text = compose(
            Milestone::Reset,
            &subject,
            &observed(Severity::Untracked, 100.0),
            ts(NOW),
        );
        assert_eq!(text.title, "Claude — Weekly");
        assert_eq!(text.body, "Limit reset · 100% left");
    }

    #[test]
    fn countdown_formats_compactly() {
        let cases = [
            (30, "<1m"),
            (42 * 60, "42m"),
            (3 * 3_600, "3h"),
            (80 * 60, "1h 20m"),
            (2 * 86_400, "2d"),
            (2 * 86_400 + 3 * 3_600 + 59, "2d 3h"),
            (-5, "<1m"),
        ];
        for (secs, expected) in cases {
            assert_eq!(countdown(SignedDuration::from_secs(secs)), expected);
        }
    }
}
