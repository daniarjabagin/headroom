use headroom_core::pace::Tone;

use super::*;
use crate::notify::evaluator::Observation;
use crate::testing::ts;

const NOW: &str = "2026-09-24T08:00:00Z";

fn observed(severity: Severity) -> Observation {
    Observation {
        remaining: 5.0,
        severity,
        tone: Tone::Critical,
        resets_at: Some(ts("2026-09-24T10:30:00Z")),
        runs_out_at: None,
    }
}

fn window_item(heading: &str, milestone: Milestone, severity: Severity, at: &str) -> HeldAlert {
    let urgency = if severity == Severity::Spent {
        Urgency::Critical
    } else {
        Urgency::Normal
    };
    HeldAlert {
        notification: Notification {
            id: format!("{heading}/{milestone:?}"),
            account_id: "codex:work".into(),
            title: "Codex · Work — Weekly".into(),
            body: "stale body".into(),
            urgency,
        },
        heading: heading.to_owned(),
        cause: Cause::Window {
            provider: "codex".into(),
            window: "weekly".into(),
            alert: Alert {
                milestone,
                threshold: 20,
            },
            observed: observed(severity),
        },
        held_at: ts(at),
    }
}

fn lapse_item(at: &str) -> HeldAlert {
    HeldAlert {
        notification: Notification {
            id: "claude:max/subscription_inactive".into(),
            account_id: "claude:max".into(),
            title: "Claude · Max — subscription inactive".into(),
            body: "Limits are unavailable until the plan is renewed.".into(),
            urgency: Urgency::Normal,
        },
        heading: "Claude · Max".into(),
        cause: Cause::Lapse,
        held_at: ts(at),
    }
}

#[test]
fn nothing_held_sends_nothing() {
    assert_eq!(compose_digest(Locale::En, Vec::new(), ts(NOW)), None);
}

#[test]
fn one_item_is_the_original_notification_with_a_fresh_body() {
    let item = window_item("Codex · Weekly", Milestone::AlmostOut, Severity::Close, NOW);
    let sent = compose_digest(Locale::En, vec![item.clone()], ts(NOW)).unwrap();
    assert_eq!(sent.id, item.notification.id);
    assert_eq!(sent.title, item.notification.title);
    assert_eq!(sent.body, "Under 20% left · resets in 2h 30m");
    let lapse = lapse_item(NOW);
    let sent = compose_digest(Locale::Ru, vec![lapse.clone()], ts(NOW)).unwrap();
    assert_eq!(sent, lapse.notification);
}

#[test]
fn items_are_listed_in_the_order_they_were_held() {
    let items = vec![
        lapse_item("2026-09-24T03:00:00Z"),
        window_item(
            "Codex · Weekly",
            Milestone::AlmostOut,
            Severity::Close,
            "2026-09-24T01:00:00Z",
        ),
        window_item(
            "Codex · Session",
            Milestone::Reset,
            Severity::Healthy,
            "2026-09-24T02:00:00Z",
        ),
    ];
    let cases = [
        (
            Locale::En,
            "Headroom — while you were away",
            "Codex · Weekly — under 20% left\nCodex · Session — limit reset\nClaude · Max — subscription inactive",
        ),
        (
            Locale::Ru,
            "Headroom — пока вас не было",
            "Codex · Weekly — осталось меньше 20%\nCodex · Session — лимит сброшен\nClaude · Max — подписка неактивна",
        ),
    ];
    for (locale, title, body) in cases {
        let sent = compose_digest(locale, items.clone(), ts(NOW)).unwrap();
        assert_eq!(sent.id, format!("summary/{}", ts(NOW).as_second()));
        assert_eq!(sent.account_id, "");
        assert_eq!(sent.title, title);
        assert_eq!(sent.body, body);
        assert_eq!(sent.urgency, Urgency::Normal);
    }
}

#[test]
fn every_milestone_has_a_short_phrase() {
    let cases = [
        (
            Milestone::AlmostOut,
            Severity::Close,
            "under 20% left",
            "осталось меньше 20%",
        ),
        (
            Milestone::CuttingItClose,
            Severity::Close,
            "close to the limit",
            "лимита едва хватит",
        ),
        (
            Milestone::WillRunOut,
            Severity::RunningOut,
            "projected to run out",
            "лимит скоро закончится",
        ),
        (
            Milestone::WillRunOut,
            Severity::Spent,
            "limit reached",
            "лимит исчерпан",
        ),
        (
            Milestone::Reset,
            Severity::Healthy,
            "limit reset",
            "лимит сброшен",
        ),
    ];
    for (milestone, severity, english, russian) in cases {
        let items = vec![window_item("A", milestone, severity, NOW), lapse_item(NOW)];
        let first_line = |locale| {
            let sent = compose_digest(locale, items.clone(), ts(NOW)).unwrap();
            sent.body.lines().next().unwrap().to_owned()
        };
        assert_eq!(first_line(Locale::En), format!("A — {english}"));
        assert_eq!(first_line(Locale::Ru), format!("A — {russian}"));
    }
}

#[test]
fn more_than_four_items_show_four_and_a_count() {
    let headings = ["A", "B", "C", "D", "E", "F", "G"];
    let items: Vec<HeldAlert> = headings
        .iter()
        .map(|h| window_item(h, Milestone::AlmostOut, Severity::Close, NOW))
        .collect();
    let cases = [(4, None), (5, Some("+1 more")), (7, Some("+3 more"))];
    for (count, last) in cases {
        let sent = compose_digest(Locale::En, items[..count].to_vec(), ts(NOW)).unwrap();
        let lines: Vec<&str> = sent.body.lines().collect();
        assert_eq!(
            lines[..4],
            [
                "A — under 20% left",
                "B — under 20% left",
                "C — under 20% left",
                "D — under 20% left"
            ]
        );
        assert_eq!(lines.get(4).copied(), last);
        assert_eq!(lines.len(), 4 + usize::from(last.is_some()));
    }
    let russian = compose_digest(Locale::Ru, items[..6].to_vec(), ts(NOW)).unwrap();
    assert_eq!(russian.body.lines().last(), Some("и ещё 2"));
}

#[test]
fn a_summary_is_as_urgent_as_its_most_urgent_item() {
    let items = vec![
        window_item("A", Milestone::AlmostOut, Severity::Close, NOW),
        window_item("B", Milestone::WillRunOut, Severity::Spent, NOW),
    ];
    let sent = compose_digest(Locale::En, items, ts(NOW)).unwrap();
    assert_eq!(sent.urgency, Urgency::Critical);
}
