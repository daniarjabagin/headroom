use headroom_core::descriptor::{ProviderDescriptor, ProviderLinks};
use headroom_core::provider::ProviderError;
use jiff::SignedDuration;

use super::*;
use crate::catalog::ProviderCatalog;
use crate::model::RefreshFailure;
use crate::state::tests::record;
use crate::testing::{CODEX, catalog, ts, usage_home_of};

const NOW: &str = "2026-09-23T10:00:00Z";
const NEXT: &str = "2026-09-23T10:01:00Z";

fn live_model() -> (Model, AccountRecord) {
    let work = record(CODEX, "work", 0);
    let mut model = Model::default();
    let home = usage_home_of(&work.reference);
    model.activity.record(&home, ts(NOW), ts(NOW));
    (model, work)
}

fn runtime(failure: Option<RefreshFailure>) -> AccountRuntime {
    AccountRuntime {
        failures: u32::from(failure.is_some()),
        failure,
        next_refresh_at: Some(ts(NEXT)),
        last_attempt: Some(ts(NOW)),
        ..AccountRuntime::default()
    }
}

fn view(model: &Model, work: &AccountRecord, runtime: &AccountRuntime) -> RefreshView {
    refresh_view(work, Some(runtime), model, &catalog(), ts(NOW))
}

#[test]
fn a_live_account_reports_activity_every_minute() {
    let (model, work) = live_model();
    let expected = RefreshView {
        mode: RefreshMode::Live,
        interval_secs: 60,
        next_at: Some(ts(NEXT)),
        reason: RefreshReason::Activity,
        last_attempt_at: Some(ts(NOW)),
    };
    assert_eq!(view(&model, &work, &runtime(None)), expected);
}

#[test]
fn an_idle_account_reports_the_schedule() {
    let (mut model, work) = live_model();
    model.settings.refresh_interval_secs = 600;
    let later = ts(NOW).checked_add(SignedDuration::from_mins(10)).unwrap();
    let idle = refresh_view(&work, Some(&runtime(None)), &model, &catalog(), later);
    assert_eq!(idle.mode, RefreshMode::Idle);
    assert_eq!(idle.interval_secs, 600);
    assert_eq!(idle.reason, RefreshReason::Schedule);
}

#[test]
fn turning_adaptive_refresh_off_keeps_a_writing_account_idle() {
    let (mut model, work) = live_model();
    model.settings.adaptive_refresh = false;
    let idle = view(&model, &work, &runtime(None));
    assert_eq!(idle.mode, RefreshMode::Idle);
    assert_eq!(idle.interval_secs, 300);
    assert_eq!(idle.reason, RefreshReason::Schedule);
}

#[test]
fn failures_report_backoff_and_provider_holds_report_hold() {
    let (model, work) = live_model();
    let down = RefreshFailure::Provider(ProviderError::Network("down".into()));
    let limited = RefreshFailure::Provider(ProviderError::rate_limited(None));
    let lapsed = RefreshFailure::Provider(ProviderError::NoSubscription {
        detail: "none".into(),
    });
    let reason = |failure| view(&model, &work, &runtime(Some(failure))).reason;
    assert_eq!(reason(down), RefreshReason::Backoff);
    assert_eq!(reason(RefreshFailure::Timeout), RefreshReason::Backoff);
    assert_eq!(reason(limited), RefreshReason::Hold);
    assert_eq!(reason(lapsed), RefreshReason::Hold);
}

#[test]
fn accounts_without_a_schedule_have_no_next_refresh() {
    let (model, work) = live_model();
    let refreshing = refresh_view(&work, None, &model, &catalog(), ts(NOW));
    assert_eq!(refreshing.next_at, None);
    assert_eq!(refreshing.mode, RefreshMode::Live);
}

#[test]
fn a_provider_minimum_slows_the_reported_interval() {
    static SLOW: ProviderDescriptor = ProviderDescriptor {
        id: CODEX,
        display_name: "Codex",
        add_account: &[],
        multi_account: false,
        local_usage: true,
        min_poll_interval: Some(SignedDuration::from_secs(180)),
        links: ProviderLinks::NONE,
    };
    let (model, work) = live_model();
    let slow = ProviderCatalog::new([&SLOW]);
    let live = refresh_view(&work, Some(&runtime(None)), &model, &slow, ts(NOW));
    assert_eq!(live.mode, RefreshMode::Live);
    assert_eq!(live.interval_secs, 180);
}
