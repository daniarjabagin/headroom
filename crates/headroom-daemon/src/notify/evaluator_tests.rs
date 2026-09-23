use super::*;
use crate::testing::ts;

const RESET: &str = "2026-09-23T12:00:00Z";
const NEXT_RESET: &str = "2026-09-23T17:00:00Z";

fn seen(remaining: f64, severity: Severity) -> Observation {
    let tone = match severity {
        Severity::Spent | Severity::RunningOut => Tone::Critical,
        Severity::Close => Tone::Warning,
        Severity::Healthy | Severity::Untracked => Tone::Good,
    };
    Observation {
        remaining,
        severity,
        tone,
        resets_at: Some(ts(RESET)),
        runs_out_at: None,
    }
}

fn after_reset(mut observation: Observation) -> Observation {
    observation.resets_at = Some(ts(NEXT_RESET));
    observation
}

fn run(steps: &[Observation]) -> Vec<Vec<Milestone>> {
    let mut state: Option<AlertState> = None;
    steps
        .iter()
        .map(|step| {
            let evaluation = evaluate(state.as_ref(), step);
            state = Some(evaluation.state);
            evaluation.alerts
        })
        .collect()
}

#[test]
fn first_observation_primes_without_alerting() {
    let evaluation = evaluate(None, &seen(5.0, Severity::RunningOut));
    assert!(evaluation.alerts.is_empty());
    assert_eq!(
        evaluation.state.fired,
        BTreeSet::from([
            Milestone::AlmostOut,
            Milestone::CuttingItClose,
            Milestone::WillRunOut
        ])
    );
    let again = evaluate(Some(&evaluation.state), &seen(4.0, Severity::Spent));
    assert!(again.alerts.is_empty());
}

#[test]
fn almost_out_fires_once_on_the_rising_edge() {
    let alerts = run(&[
        seen(20.0, Severity::Healthy),
        seen(9.0, Severity::Healthy),
        seen(8.0, Severity::Healthy),
    ]);
    assert_eq!(alerts, [vec![], vec![Milestone::AlmostOut], vec![]]);
}

#[test]
fn cutting_it_close_fires_when_severity_rises_to_close() {
    let alerts = run(&[
        seen(60.0, Severity::Healthy),
        seen(50.0, Severity::Close),
        seen(45.0, Severity::Close),
    ]);
    assert_eq!(alerts, [vec![], vec![Milestone::CuttingItClose], vec![]]);
}

#[test]
fn will_run_out_fires_on_running_out_and_spent() {
    let alerts = run(&[
        seen(60.0, Severity::Close),
        seen(40.0, Severity::RunningOut),
    ]);
    assert_eq!(alerts, [vec![], vec![Milestone::WillRunOut]]);
    let spent = run(&[seen(60.0, Severity::Healthy), seen(0.0, Severity::Spent)]);
    assert_eq!(spent[1], [Milestone::WillRunOut, Milestone::AlmostOut]);
}

#[test]
fn jumping_straight_to_running_out_skips_cutting_it_close() {
    let alerts = run(&[
        seen(60.0, Severity::Healthy),
        seen(40.0, Severity::RunningOut),
        seen(38.0, Severity::Close),
    ]);
    assert_eq!(alerts, [vec![], vec![Milestone::WillRunOut], vec![]]);
}

#[test]
fn almost_out_re_arms_only_above_fifteen_percent() {
    let alerts = run(&[
        seen(20.0, Severity::Healthy),
        seen(9.0, Severity::Healthy),
        seen(12.0, Severity::Healthy),
        seen(9.0, Severity::Healthy),
        seen(15.0, Severity::Healthy),
        seen(9.0, Severity::Healthy),
    ]);
    let fired: Vec<usize> = alerts.iter().map(Vec::len).collect();
    assert_eq!(fired, [0, 1, 0, 0, 0, 1]);
}

#[test]
fn severity_milestones_re_arm_only_after_dropping_to_healthy() {
    let alerts = run(&[
        seen(60.0, Severity::Healthy),
        seen(50.0, Severity::RunningOut),
        seen(50.0, Severity::Close),
        seen(50.0, Severity::RunningOut),
        seen(50.0, Severity::Healthy),
        seen(50.0, Severity::RunningOut),
        seen(50.0, Severity::Healthy),
        seen(50.0, Severity::Close),
    ]);
    assert_eq!(
        alerts,
        [
            vec![],
            vec![Milestone::WillRunOut],
            vec![],
            vec![],
            vec![],
            vec![Milestone::WillRunOut],
            vec![],
            vec![Milestone::CuttingItClose]
        ]
    );
}

#[test]
fn reset_of_a_warning_window_alerts_and_clears_milestones() {
    let alerts = run(&[
        seen(60.0, Severity::Healthy),
        seen(8.0, Severity::Close),
        after_reset(seen(100.0, Severity::Untracked)),
        after_reset(seen(8.0, Severity::Close)),
    ]);
    assert_eq!(
        alerts,
        [
            vec![],
            vec![Milestone::CuttingItClose, Milestone::AlmostOut],
            vec![Milestone::Reset],
            vec![Milestone::CuttingItClose, Milestone::AlmostOut]
        ]
    );
}

#[test]
fn reset_of_a_good_window_is_silent() {
    let alerts = run(&[
        seen(60.0, Severity::Healthy),
        after_reset(seen(100.0, Severity::Untracked)),
    ]);
    assert_eq!(alerts, [vec![], vec![]]);
}

#[test]
fn reset_time_jitter_is_not_a_reset() {
    let mut jittered = seen(9.0, Severity::Close);
    jittered.resets_at = Some(ts("2026-09-23T12:00:00.900Z"));
    let alerts = run(&[seen(9.0, Severity::Close), jittered]);
    assert_eq!(alerts, [vec![], vec![]]);
}

#[test]
fn rolled_back_milestone_fires_again_next_time() {
    let primed = evaluate(None, &seen(20.0, Severity::Healthy)).state;
    let mut first = evaluate(Some(&primed), &seen(9.0, Severity::Healthy));
    assert_eq!(first.alerts, [Milestone::AlmostOut]);
    rollback(&mut first.state, Milestone::AlmostOut);
    let retry = evaluate(Some(&first.state), &seen(9.0, Severity::Healthy));
    assert_eq!(retry.alerts, [Milestone::AlmostOut]);
}

#[test]
fn rolled_back_reset_is_retried() {
    let primed = evaluate(None, &seen(50.0, Severity::Close)).state;
    let mut reset = evaluate(
        Some(&primed),
        &after_reset(seen(100.0, Severity::Untracked)),
    );
    assert_eq!(reset.alerts, [Milestone::Reset]);
    rollback(&mut reset.state, Milestone::Reset);
    let retry = evaluate(
        Some(&reset.state),
        &after_reset(seen(100.0, Severity::Untracked)),
    );
    assert_eq!(retry.alerts, [Milestone::Reset]);
    let settled = evaluate(
        Some(&retry.state),
        &after_reset(seen(100.0, Severity::Untracked)),
    );
    assert!(settled.alerts.is_empty());
}

#[test]
fn state_serializes_compactly() {
    let state = evaluate(None, &seen(5.0, Severity::Close)).state;
    let json = serde_json::to_string(&state).unwrap();
    assert_eq!(
        json,
        r#"{"resets_at":"2026-09-23T12:00:00Z","tone":"warning","fired":["cutting_it_close","almost_out"],"reset_owed":false}"#
    );
    assert_eq!(serde_json::from_str::<AlertState>(&json).unwrap(), state);
}

#[test]
fn observation_uses_core_pace_and_tone() {
    let window = crate::testing::session(55.0, RESET);
    let observed = Observation::of(&window, ts("2026-09-23T10:00:00Z"));
    assert_eq!(observed.severity, Severity::Close);
    assert_eq!(observed.tone, Tone::Warning);
    assert!((observed.remaining - 45.0).abs() < f64::EPSILON);
}
