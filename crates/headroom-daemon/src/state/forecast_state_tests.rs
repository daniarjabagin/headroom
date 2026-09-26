use headroom_core::account::AccountId;
use headroom_core::history::UsageSample;
use headroom_core::pace::{Basis, Severity, Tone};
use headroom_core::units::Percent;

use super::payload::PaceView;
use super::tests::{assemble_sample, sample_model};
use crate::model::Model;
use crate::quota_history::QuotaHistory;
use crate::testing::{ts, usage_home_of};

const NOW: &str = "2026-09-23T10:00:00Z";

fn row(id: &AccountId, window: &str, at: &str, used: f64) -> (AccountId, String, UsageSample) {
    let sample = UsageSample {
        at: ts(at),
        used: Percent::new(used),
    };
    (id.clone(), window.to_owned(), sample)
}

fn model_with_history() -> Model {
    let mut model = sample_model();
    let work = model.accounts[0].id().clone();
    let claude = model.accounts[1].id().clone();
    model.history = QuotaHistory::from_rows(vec![
        row(&work, "session", "2026-09-23T09:49:00Z", 52.0),
        row(&work, "session", "2026-09-23T09:52:00Z", 53.0),
        row(&work, "session", "2026-09-23T09:55:00Z", 54.0),
        row(&work, "session", "2026-09-23T09:58:00Z", 55.0),
        row(&work, "weekly", "2026-09-23T09:00:00Z", 27.0),
        row(&work, "weekly", "2026-09-23T09:20:00Z", 28.0),
        row(&work, "weekly", "2026-09-23T09:40:00Z", 29.0),
        row(&work, "weekly", "2026-09-23T09:58:00Z", 30.0),
        row(&claude, "session", "2026-09-23T08:25:00Z", 80.0),
        row(&claude, "session", "2026-09-23T08:30:00Z", 84.0),
        row(&claude, "session", "2026-09-23T08:35:00Z", 88.0),
        row(&claude, "session", "2026-09-23T08:40:00Z", 92.0),
    ]);
    model
}

fn paces(model: &Model) -> Vec<(PaceView, Tone)> {
    assemble_sample(model)
        .accounts
        .iter()
        .flat_map(|account| account.windows.iter())
        .map(|window| (window.pace.clone(), window.tone))
        .collect()
}

#[test]
fn without_history_every_tracked_window_uses_the_window_average() {
    let bases: Vec<_> = paces(&sample_model())
        .into_iter()
        .map(|(pace, _)| (pace.basis, pace.active_left_seconds))
        .collect();
    let window = (Some(Basis::Window), None);
    assert_eq!(bases, vec![window, window, window]);
}

#[test]
fn history_drives_recent_and_paused_forecasts() {
    let paces = paces(&model_with_history());
    let (session, _) = &paces[0];
    assert_eq!(session.basis, Some(Basis::Recent));
    assert_eq!(session.severity, Severity::Close);
    assert_eq!(session.projected_percent, Some(95.0));
    assert_eq!(session.spare_percent, Some(5.0));
    let (weekly, weekly_tone) = &paces[1];
    assert_eq!(weekly.basis, Some(Basis::Window));
    assert_eq!(weekly.severity, Severity::Healthy);
    assert_eq!(weekly.runs_out_at, None);
    assert_eq!(*weekly_tone, Tone::Good);
    let (claude, _) = &paces[2];
    assert_eq!(claude.basis, Some(Basis::Paused));
    assert_eq!(claude.severity, Severity::RunningOut);
    assert_eq!(claude.runs_out_at, None);
    assert_eq!(claude.active_left_seconds, Some(600));
}

#[test]
fn live_activity_resumes_a_paused_forecast() {
    let mut model = model_with_history();
    let home = usage_home_of(&model.accounts[1].reference);
    model.activity.record(&home, ts(NOW), ts(NOW));
    let (claude, _) = &paces(&model)[2];
    assert_eq!(claude.basis, Some(Basis::Window));
    assert_eq!(claude.active_left_seconds, None);
    assert!(claude.runs_out_at.is_some());
}

#[test]
fn an_account_without_local_logs_is_never_paused() {
    let mut model = model_with_history();
    let home = usage_home_of(&model.accounts[1].reference);
    model.usage_homes.remove(&home);
    let (claude, _) = &paces(&model)[2];
    assert_eq!(claude.basis, Some(Basis::Window));
    assert_eq!(claude.active_left_seconds, None);
}

#[test]
fn a_failing_account_with_a_stale_snapshot_does_not_drift_into_a_pause() {
    let mut model = sample_model();
    let claude = model.accounts[1].id().clone();
    model.history = QuotaHistory::from_rows(vec![
        row(&claude, "session", "2026-09-23T08:45:00Z", 80.0),
        row(&claude, "session", "2026-09-23T08:50:00Z", 84.0),
        row(&claude, "session", "2026-09-23T08:55:00Z", 88.0),
        row(&claude, "session", "2026-09-23T09:00:00Z", 92.0),
    ]);
    let (claude, _) = &paces(&model)[2];
    assert_eq!(claude.basis, Some(Basis::Window));
}
