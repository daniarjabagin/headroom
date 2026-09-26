use headroom_core::account::AccountRef;
use jiff::Timestamp;

use super::*;
use crate::activity;
use crate::clock::Clock;
use crate::model::AccountRuntime;
use crate::scheduler::policy::{effective_interval, live_delay};
use crate::settings::Settings;
use crate::state::payload::{RefreshMode, RefreshReason, RefreshView};
use crate::testing::{FakeProvider, ts, usage_home_of};

fn secs(value: i64) -> SignedDuration {
    SignedDuration::from_secs(value)
}

fn write_logs(harness: &Harness) {
    let home = usage_home_of(&account(CODEX, "work"));
    activity::record_write(&harness.core, &home, None);
}

fn runtime(harness: &Harness, id: &AccountId) -> AccountRuntime {
    harness.core.model().runtime[id].clone()
}

fn settled(harness: &Harness, provider: &ScriptedProvider, calls: usize) -> bool {
    let model = harness.core.model();
    let scheduled = model
        .runtime
        .get(&work_id())
        .is_some_and(|r| !r.refreshing && r.next_refresh_at.is_some());
    provider.calls() == calls && scheduled
}

fn refresh_view(harness: &Harness) -> RefreshView {
    harness.core.state().accounts[0].refresh.unwrap()
}

fn after(harness: &Harness, delay: i64) -> Option<Timestamp> {
    harness.clock.now().checked_add(secs(delay)).ok()
}

async fn started_live(provider: &Arc<ScriptedProvider>) -> (Harness, Scheduler) {
    let (harness, scheduler) = start(provider).await;
    eventually(|| settled(&harness, provider, 1)).await;
    write_logs(&harness);
    (harness, scheduler)
}

#[test]
fn live_accounts_poll_every_minute_only_with_adaptive_refresh_on() {
    let mut settings = Settings::default();
    assert_eq!(effective_interval(&settings, true), secs(60));
    assert_eq!(effective_interval(&settings, false), secs(300));
    settings.adaptive_refresh = false;
    assert_eq!(effective_interval(&settings, true), secs(300));
    settings.refresh_interval_secs = 1;
    assert_eq!(effective_interval(&settings, false), secs(60));
}

#[test]
fn the_live_interval_never_drops_under_a_minute_with_jitter() {
    assert_eq!(policy::next_delay(Ok(()), 0, secs(60), 0.0), secs(60));
    assert_eq!(policy::next_delay(Ok(()), 0, secs(60), 1.0), secs(66));
}

#[test]
fn going_live_never_shortens_holds_backoff_or_a_refresh_in_flight() {
    let now = ts("2026-09-23T10:00:00Z");
    let recent = AccountRuntime {
        last_attempt: Some(ts("2026-09-23T09:59:40Z")),
        ..AccountRuntime::default()
    };
    assert_eq!(live_delay(None, secs(60), now), Some(SignedDuration::ZERO));
    assert_eq!(live_delay(Some(&recent), secs(60), now), Some(secs(40)));
    let failed = AccountRuntime {
        failure: Some(crate::model::RefreshFailure::Timeout),
        ..recent.clone()
    };
    let held = AccountRuntime {
        hold_until: Some(ts("2026-09-23T10:05:00Z")),
        ..recent.clone()
    };
    let refreshing = AccountRuntime {
        refreshing: true,
        ..recent
    };
    assert_eq!(live_delay(Some(&failed), secs(60), now), None);
    assert_eq!(live_delay(Some(&held), secs(60), now), None);
    assert_eq!(live_delay(Some(&refreshing), secs(60), now), None);
}

#[tokio::test(start_paused = true)]
async fn a_live_account_refreshes_every_minute() {
    let provider = Arc::new(ScriptedProvider::new(Vec::new()));
    let (harness, _scheduler) = started_live(&provider).await;
    let expected = RefreshView {
        mode: RefreshMode::Live,
        interval_secs: 60,
        next_at: after(&harness, 60),
        reason: RefreshReason::Activity,
        last_attempt_at: None,
    };
    let scheduled = || RefreshView {
        last_attempt_at: None,
        ..refresh_view(&harness)
    };
    eventually_virtual(|| scheduled() == expected).await;
    eventually_virtual(|| provider.calls() == 4).await;
    assert_eq!(provider.gaps(), [60, 60, 60]);
}

#[tokio::test(start_paused = true)]
async fn ten_quiet_minutes_return_to_the_normal_interval() {
    let provider = Arc::new(ScriptedProvider::new(Vec::new()));
    let (harness, _scheduler) = started_live(&provider).await;
    eventually_virtual(|| settled(&harness, &provider, 3)).await;
    harness.clock.advance(SignedDuration::from_mins(10));
    eventually_virtual(|| settled(&harness, &provider, 4)).await;
    let view = refresh_view(&harness);
    assert_eq!(view.mode, RefreshMode::Idle);
    assert_eq!(view.reason, RefreshReason::Schedule);
    assert_eq!(view.next_at, after(&harness, 300));
    eventually_virtual(|| provider.calls() == 5).await;
    assert_eq!(provider.gaps(), [60, 60, 60, 300]);
}

#[tokio::test(start_paused = true)]
async fn with_adaptive_refresh_off_writes_keep_the_normal_interval() {
    let provider = Arc::new(ScriptedProvider::new(Vec::new()));
    let (harness, _scheduler) = start(&provider).await;
    harness.core.model().settings.adaptive_refresh = false;
    eventually_virtual(|| settled(&harness, &provider, 1)).await;
    write_logs(&harness);
    let view = refresh_view(&harness);
    assert_eq!((view.mode, view.interval_secs), (RefreshMode::Idle, 300));
    assert_eq!(view.reason, RefreshReason::Schedule);
    eventually_virtual(|| provider.calls() == 3).await;
    assert_eq!(provider.gaps(), [300, 300]);
}

#[tokio::test(start_paused = true)]
async fn going_live_keeps_a_rate_limit_hold() {
    let limited = Err(ProviderError::rate_limited(Some(secs(120))));
    let provider = Arc::new(ScriptedProvider::new(vec![limited]));
    let (harness, _scheduler) = started_live(&provider).await;
    let held = runtime(&harness, &work_id()).hold_until;
    let view = refresh_view(&harness);
    assert_eq!(
        (view.mode, view.reason),
        (RefreshMode::Live, RefreshReason::Hold)
    );
    assert_eq!(view.next_at, held);
    eventually_virtual(|| provider.calls() == 3).await;
    assert_eq!(provider.gaps(), [120, 60]);
}

#[tokio::test(start_paused = true)]
async fn going_live_keeps_the_backoff() {
    let down = || Err(ProviderError::Network("down".into()));
    let provider = Arc::new(ScriptedProvider::new(vec![down(), down()]));
    let (harness, _scheduler) = started_live(&provider).await;
    eventually_virtual(|| settled(&harness, &provider, 2)).await;
    let view = refresh_view(&harness);
    assert_eq!(view.reason, RefreshReason::Backoff);
    assert_eq!(view.next_at, after(&harness, 120));
    eventually_virtual(|| provider.calls() == 4).await;
    assert_eq!(provider.gaps(), [60, 120, 60]);
}

#[tokio::test(start_paused = true)]
async fn going_live_refreshes_at_once_when_the_last_refresh_is_a_minute_old() {
    let provider = Arc::new(ScriptedProvider::new(Vec::new()));
    let (harness, _scheduler) = start(&provider).await;
    eventually_virtual(|| settled(&harness, &provider, 1)).await;
    harness.clock.advance(secs(120));
    let started = tokio::time::Instant::now();
    write_logs(&harness);
    eventually(|| provider.calls() == 2).await;
    assert_eq!(started.elapsed().as_secs(), 0);
}

fn twin_accounts() -> Arc<FakeProvider> {
    let work = account(CODEX, "work");
    let personal = account(CODEX, "personal");
    Arc::new(FakeProvider::new(CODEX, vec![work, personal], good()))
}

fn last_attempts(harness: &Harness, accounts: &[AccountRef]) -> Vec<Option<Timestamp>> {
    let model = harness.core.model();
    accounts
        .iter()
        .map(|a| model.runtime.get(&a.id).and_then(|r| r.last_attempt))
        .collect()
}

#[tokio::test(start_paused = true)]
async fn accounts_sharing_a_usage_home_go_live_together() {
    let provider = twin_accounts();
    let dynamic: Arc<dyn Provider> = provider.clone();
    let harness = harness(vec![dynamic]).await;
    let accounts = harness.core.active_accounts();
    assert_eq!(accounts.len(), 2);
    let mut scheduler = Scheduler::new(harness.core.clone());
    scheduler.sync(&accounts, FirstRefresh::Scheduled);
    let first = Some(harness.clock.now());
    eventually_virtual(|| last_attempts(&harness, &accounts) == [first, first]).await;
    harness.clock.advance(secs(120));
    write_logs(&harness);
    let live = Some(harness.clock.now());
    eventually_virtual(|| last_attempts(&harness, &accounts) == [live, live]).await;
    let modes: Vec<_> = harness
        .core
        .state()
        .accounts
        .iter()
        .map(|a| a.refresh.map(|r| r.mode))
        .collect();
    assert_eq!(modes, [Some(RefreshMode::Live), Some(RefreshMode::Live)]);
}
