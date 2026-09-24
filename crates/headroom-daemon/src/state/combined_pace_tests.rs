use headroom_core::pace::{Severity, Tone};

use super::*;
use crate::state::test_views::{tracked, window};
use crate::testing::ts;

const RESET: &str = "2026-09-23T12:00:00Z";

fn used(percent: f64) -> WindowView {
    window("weekly", percent, RESET)
}

fn projecting(percent: f64, projected: f64) -> WindowView {
    tracked(used(percent), Severity::Healthy, projected)
}

fn segment((used_percent, projected): (f64, Option<f64>)) -> WindowView {
    match projected {
        Some(projected) => projecting(used_percent, projected),
        None => used(used_percent),
    }
}

type Segments = &'static [(f64, Option<f64>)];
type Case = (
    &'static str,
    Segments,
    Severity,
    Tone,
    Option<f64>,
    Option<f64>,
);

const CASES: &[Case] = &[
    (
        "healthy sums projections",
        &[(20.0, Some(40.0)), (30.0, Some(60.0))],
        Severity::Healthy,
        Tone::Good,
        Some(100.0),
        Some(100.0),
    ),
    (
        "exactly 90 % of capacity is healthy",
        &[(40.0, Some(90.0)), (40.0, Some(90.0))],
        Severity::Healthy,
        Tone::Good,
        Some(180.0),
        Some(20.0),
    ),
    (
        "close",
        &[(50.0, Some(95.0)), (45.0, Some(90.0))],
        Severity::Close,
        Tone::Warning,
        Some(185.0),
        Some(15.0),
    ),
    (
        "running out below 90 % used warns",
        &[(60.0, Some(150.0)), (40.0, Some(80.0))],
        Severity::RunningOut,
        Tone::Warning,
        Some(230.0),
        None,
    ),
    (
        "running out at 90 % used is critical",
        &[(95.0, Some(120.0)), (85.0, Some(100.0))],
        Severity::RunningOut,
        Tone::Critical,
        Some(220.0),
        None,
    ),
    (
        "untracked segments count their current use",
        &[(25.0, Some(50.0)), (70.0, None)],
        Severity::Healthy,
        Tone::Good,
        Some(120.0),
        Some(80.0),
    ),
    (
        "spent segment counts as fully used",
        &[(100.0, None), (45.0, Some(90.0))],
        Severity::Close,
        Tone::Warning,
        Some(190.0),
        Some(10.0),
    ),
    (
        "low combined use is not tracked",
        &[(2.0, Some(190.0)), (2.0, None)],
        Severity::Untracked,
        Tone::Good,
        None,
        None,
    ),
    (
        "all untracked falls back to level good",
        &[(20.0, None), (30.0, None)],
        Severity::Untracked,
        Tone::Good,
        None,
        None,
    ),
    (
        "all untracked at 80 % used warns",
        &[(85.0, None), (75.0, None)],
        Severity::Untracked,
        Tone::Warning,
        None,
        None,
    ),
    (
        "all untracked at 90 % used is critical",
        &[(95.0, None), (85.0, None)],
        Severity::Untracked,
        Tone::Critical,
        None,
        None,
    ),
    (
        "one spent account alone is not critical",
        &[(100.0, None), (20.0, None)],
        Severity::Untracked,
        Tone::Good,
        None,
        None,
    ),
    (
        "every account spent",
        &[(100.0, None), (100.0, None)],
        Severity::Spent,
        Tone::Critical,
        None,
        None,
    ),
];

#[test]
fn combined_pace_compares_summed_projections_with_capacity() {
    for &(name, segments, severity, expected_tone, projected, spare) in CASES {
        let windows: Vec<WindowView> = segments.iter().copied().map(segment).collect();
        let (pace, tone) = combined_pace(&windows.iter().collect::<Vec<_>>());
        assert_eq!(pace.severity, severity, "{name}");
        assert_eq!(tone, expected_tone, "{name}");
        assert_eq!(pace.projected_percent, projected, "{name}");
        assert_eq!(pace.spare_percent, spare, "{name}");
        assert_eq!(pace.runs_out_at, None, "{name}");
    }
}

#[test]
fn even_pace_is_summed_only_when_every_segment_has_one() {
    let both = [projecting(20.0, 40.0), projecting(30.0, 60.0)];
    let (pace, _) = combined_pace(&both.iter().collect::<Vec<_>>());
    assert_eq!(pace.even_pace_percent, Some(100.0));
    let partial = [projecting(20.0, 40.0), used(30.0)];
    let (pace, _) = combined_pace(&partial.iter().collect::<Vec<_>>());
    assert_eq!(pace.even_pace_percent, None);
}

#[test]
fn three_accounts_share_a_capacity_of_three_hundred() {
    let segments = [
        projecting(60.0, 100.0),
        projecting(60.0, 100.0),
        projecting(60.0, 71.0),
    ];
    let (pace, tone) = combined_pace(&segments.iter().collect::<Vec<_>>());
    assert_eq!(pace.severity, Severity::Close);
    assert_eq!(tone, Tone::Warning);
    assert_eq!(pace.spare_percent, Some(29.0));
}

#[test]
fn a_window_only_one_account_has_keeps_that_account_pace_and_tone() {
    let mut only = tracked(used(60.0), Severity::RunningOut, 130.0);
    only.pace.runs_out_at = Some(ts("2026-09-23T11:00:00Z"));
    only.tone = Tone::Critical;
    let (pace, tone) = combined_pace(&[&only]);
    assert_eq!(pace, only.pace);
    assert_eq!(tone, Tone::Critical);
}
