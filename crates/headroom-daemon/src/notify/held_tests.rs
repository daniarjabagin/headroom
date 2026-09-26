use std::collections::BTreeSet;

use headroom_core::pace::{Severity, Tone};

use super::*;
use crate::testing::ts;

const NOW: &str = "2026-09-24T08:00:00Z";
const RESET: &str = "2026-09-24T12:00:00Z";

fn observed(resets_at: &str) -> Observation {
    Observation {
        remaining: 5.0,
        severity: Severity::Close,
        tone: Tone::Warning,
        resets_at: Some(ts(resets_at)),
        runs_out_at: None,
        paused: false,
    }
}

fn state(resets_at: &str, fired: &[Milestone]) -> AlertState {
    AlertState {
        resets_at: Some(ts(resets_at)),
        tone: Tone::Warning,
        fired: fired.iter().copied().collect::<BTreeSet<_>>(),
        reset_owed: false,
    }
}

#[test]
fn held_window_alerts_hold_only_while_their_condition_does() {
    let fired = state(RESET, &[Milestone::AlmostOut]);
    let rearmed = state(RESET, &[]);
    let moved = state("2026-09-24T17:00:00Z", &[Milestone::AlmostOut]);
    let cases = [
        (Milestone::AlmostOut, RESET, Some(&fired), true),
        (Milestone::AlmostOut, RESET, Some(&rearmed), false),
        (Milestone::AlmostOut, RESET, Some(&moved), false),
        (Milestone::AlmostOut, RESET, None, false),
        (
            Milestone::AlmostOut,
            "2026-09-24T07:59:00Z",
            Some(&fired),
            false,
        ),
        (Milestone::Reset, RESET, Some(&rearmed), true),
        (Milestone::Reset, RESET, Some(&moved), false),
    ];
    for (milestone, resets_at, current, expected) in cases {
        let holds = window_still_holds(milestone, &observed(resets_at), current, ts(NOW));
        assert_eq!(holds, expected, "{milestone:?} {resets_at} {current:?}");
    }
}

#[test]
fn held_alerts_round_trip_through_json() {
    let alert = HeldAlert {
        notification: Notification {
            id: "codex:work/weekly/almost_out".into(),
            account_id: "codex:work".into(),
            title: "Codex · Work — Weekly".into(),
            body: "Under 10% left · resets in 4h".into(),
            urgency: crate::notify::Urgency::Normal,
        },
        heading: "Codex · Work · Weekly".into(),
        cause: Cause::Window {
            provider: "codex".into(),
            window: "weekly".into(),
            alert: Alert {
                milestone: Milestone::AlmostOut,
                threshold: 10,
            },
            observed: observed(RESET),
        },
        held_at: ts(NOW),
    };
    let json = serde_json::to_string(&alert).unwrap();
    assert_eq!(serde_json::from_str::<HeldAlert>(&json).unwrap(), alert);
    assert_eq!(alert.account(), AccountId("codex:work".into()));
}
