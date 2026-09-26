use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use async_trait::async_trait;
use headroom_core::account::AccountId;
use headroom_core::cursor::LogCursors;
use headroom_core::descriptor::ProviderDescriptor;
use headroom_core::event::UsageEvent;
use headroom_core::provider::{Provider, ProviderError};
use headroom_core::quota::LimitsSnapshot;
use jiff::SignedDuration;
use tokio::sync::Semaphore;
use tokio::time::Instant;

use super::*;
use crate::error::CommandError;
use crate::state::payload::AccountStatus;
use crate::storage::{accounts, lapses, snapshots};
use crate::testing::{CODEX, CODEX_DESCRIPTOR};
use crate::testing::{
    Harness, account, eventually, eventually_virtual, harness, session, snapshot,
};

type Outcome = Result<LimitsSnapshot, ProviderError>;

struct ScriptedProvider {
    account: AccountRef,
    script: Mutex<VecDeque<Outcome>>,
    calls: AtomicUsize,
    started: Mutex<Vec<Instant>>,
    gate: Option<Semaphore>,
    stall: Option<Duration>,
}

impl ScriptedProvider {
    fn new(script: Vec<Outcome>) -> ScriptedProvider {
        ScriptedProvider {
            account: account(CODEX, "work"),
            script: Mutex::new(script.into()),
            calls: AtomicUsize::new(0),
            started: Mutex::new(Vec::new()),
            gate: None,
            stall: None,
        }
    }

    fn calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }

    fn gaps(&self) -> Vec<u64> {
        let started = self.started.lock().unwrap();
        started
            .windows(2)
            .map(|pair| (pair[1] - pair[0]).as_secs())
            .collect()
    }
}

fn good() -> LimitsSnapshot {
    snapshot(
        vec![session(20.0, "2026-09-23T12:00:00Z")],
        "2026-09-23T10:00:00Z",
    )
}

#[async_trait]
impl Provider for ScriptedProvider {
    fn descriptor(&self) -> &'static ProviderDescriptor {
        &CODEX_DESCRIPTOR
    }

    async fn discover(&self) -> Result<Vec<AccountRef>, ProviderError> {
        Ok(vec![self.account.clone()])
    }

    async fn usage_homes(&self) -> Result<Vec<PathBuf>, ProviderError> {
        Ok(Vec::new())
    }

    async fn fetch_limits(&self, _account: &AccountRef) -> Outcome {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.started.lock().unwrap().push(Instant::now());
        if let Some(gate) = &self.gate {
            gate.acquire().await.unwrap().forget();
        }
        if let Some(stall) = self.stall {
            tokio::time::sleep(stall).await;
        }
        self.script
            .lock()
            .unwrap()
            .pop_front()
            .unwrap_or_else(|| Ok(good()))
    }

    fn read_usage(
        &self,
        _home: &Path,
        _cursors: &mut LogCursors,
    ) -> Result<Vec<UsageEvent>, ProviderError> {
        Ok(Vec::new())
    }
}

async fn start(provider: &Arc<ScriptedProvider>) -> (Harness, Scheduler) {
    let dynamic: Arc<dyn Provider> = provider.clone();
    let harness = harness(vec![dynamic]).await;
    let mut scheduler = Scheduler::new(harness.core.clone());
    scheduler.sync(&harness.core.active_accounts(), FirstRefresh::Scheduled);
    (harness, scheduler)
}

fn work_id() -> AccountId {
    account(CODEX, "work").id
}

fn status(harness: &Harness) -> AccountStatus {
    harness.core.state().accounts[0].status
}

#[tokio::test]
async fn forced_refreshes_during_a_flight_queue_exactly_one_follow_up() {
    let mut scripted = ScriptedProvider::new(Vec::new());
    scripted.gate = Some(Semaphore::new(0));
    let provider = Arc::new(scripted);
    let (harness, _scheduler) = start(&provider).await;
    eventually(|| provider.calls() == 1).await;
    assert_eq!(status(&harness), AccountStatus::Refreshing);
    for _ in 0..3 {
        harness.core.refresh("codex:work").unwrap();
    }
    provider.gate.as_ref().unwrap().add_permits(10);
    eventually(|| provider.calls() == 2).await;
    eventually(|| status(&harness) == AccountStatus::Fresh).await;
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(provider.calls(), 2);
}

#[tokio::test]
async fn soft_refresh_skips_accounts_refreshed_within_a_minute() {
    let provider = Arc::new(ScriptedProvider::new(Vec::new()));
    let (harness, _scheduler) = start(&provider).await;
    eventually(|| provider.calls() == 1 && status(&harness) == AccountStatus::Fresh).await;
    harness.core.refresh("").unwrap();
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert_eq!(provider.calls(), 1);
    harness.clock.advance(SignedDuration::from_secs(61));
    harness.core.refresh("").unwrap();
    eventually(|| provider.calls() == 2).await;
}

#[tokio::test]
async fn refresh_now_bypasses_the_minute_rule_and_shows_refreshing_at_once() {
    let mut scripted = ScriptedProvider::new(Vec::new());
    scripted.gate = Some(Semaphore::new(1));
    let provider = Arc::new(scripted);
    let (harness, _scheduler) = start(&provider).await;
    eventually(|| provider.calls() == 1 && status(&harness) == AccountStatus::Fresh).await;
    harness.core.refresh_now();
    assert_eq!(status(&harness), AccountStatus::Refreshing);
    assert_eq!(harness.core.state().next_refresh_at, None);
    eventually(|| provider.calls() == 2).await;
    provider.gate.as_ref().unwrap().add_permits(10);
    eventually(|| status(&harness) == AccountStatus::Fresh).await;
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert_eq!(provider.calls(), 2);
}

#[tokio::test]
async fn refresh_now_keeps_rate_limit_holds_and_rechecks_lapses() {
    let limited = Err(ProviderError::rate_limited(Some(
        SignedDuration::from_secs(120),
    )));
    let provider = Arc::new(ScriptedProvider::new(vec![limited, lapsed()]));
    let (harness, _scheduler) = start(&provider).await;
    eventually(|| provider.calls() == 1 && status(&harness) == AccountStatus::Error).await;
    harness.core.refresh_now();
    assert_eq!(status(&harness), AccountStatus::Error);
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert_eq!(provider.calls(), 1);
    harness.clock.advance(SignedDuration::from_secs(121));
    harness.core.refresh_now();
    eventually(|| status(&harness) == AccountStatus::NoSubscription).await;
    harness.core.refresh_now();
    eventually(|| provider.calls() == 3 && status(&harness) == AccountStatus::Fresh).await;
}

#[tokio::test]
async fn refreshing_one_account_bypasses_the_minute_rule_and_shows_refreshing_at_once() {
    let mut scripted = ScriptedProvider::new(Vec::new());
    scripted.gate = Some(Semaphore::new(1));
    let provider = Arc::new(scripted);
    let (harness, _scheduler) = start(&provider).await;
    eventually(|| provider.calls() == 1 && status(&harness) == AccountStatus::Fresh).await;
    harness.core.refresh("codex:work").unwrap();
    assert_eq!(status(&harness), AccountStatus::Refreshing);
    assert_eq!(harness.core.state().next_refresh_at, None);
    eventually(|| provider.calls() == 2).await;
    provider.gate.as_ref().unwrap().add_permits(10);
    eventually(|| status(&harness) == AccountStatus::Fresh).await;
}

#[tokio::test]
async fn refreshing_a_signed_out_account_retries_it() {
    let provider = Arc::new(ScriptedProvider::new(vec![Err(
        ProviderError::SignInExpired,
    )]));
    let (harness, _scheduler) = start(&provider).await;
    eventually(|| provider.calls() == 1 && status(&harness) == AccountStatus::SignedOut).await;
    harness.core.refresh("codex:work").unwrap();
    assert_eq!(status(&harness), AccountStatus::Refreshing);
    eventually(|| provider.calls() == 2 && status(&harness) == AccountStatus::Fresh).await;
}

#[tokio::test]
async fn refreshing_one_account_keeps_rate_limit_holds() {
    let limited = Err(ProviderError::rate_limited(Some(
        SignedDuration::from_secs(120),
    )));
    let provider = Arc::new(ScriptedProvider::new(vec![limited]));
    let (harness, _scheduler) = start(&provider).await;
    eventually(|| provider.calls() == 1 && status(&harness) == AccountStatus::Error).await;
    harness.core.refresh("codex:work").unwrap();
    assert_eq!(status(&harness), AccountStatus::Error);
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert_eq!(provider.calls(), 1);
    harness.clock.advance(SignedDuration::from_secs(121));
    harness.core.refresh("codex:work").unwrap();
    eventually(|| provider.calls() == 2 && status(&harness) == AccountStatus::Fresh).await;
}

#[tokio::test]
async fn refreshing_an_unknown_account_is_rejected() {
    let provider = Arc::new(ScriptedProvider::new(Vec::new()));
    let (harness, _scheduler) = start(&provider).await;
    assert!(matches!(
        harness.core.refresh("codex:nobody"),
        Err(CommandError::UnknownAccount(_))
    ));
}

#[tokio::test(start_paused = true)]
async fn failures_back_off_exponentially() {
    let down = || Err(ProviderError::Network("down".into()));
    let provider = Arc::new(ScriptedProvider::new(vec![down(), down(), down(), down()]));
    let (_harness, _scheduler) = start(&provider).await;
    eventually_virtual(|| provider.calls() == 5).await;
    assert_eq!(provider.gaps(), [60, 120, 240, 480]);
}

#[tokio::test(start_paused = true)]
async fn rate_limits_honour_retry_after_then_back_off_and_resume_the_interval() {
    let limited = Err(ProviderError::rate_limited(Some(
        SignedDuration::from_secs(120),
    )));
    let unspecified = Err(ProviderError::rate_limited(None));
    let provider = Arc::new(ScriptedProvider::new(vec![limited, unspecified]));
    let (harness, _scheduler) = start(&provider).await;
    eventually_virtual(|| provider.calls() == 4).await;
    assert_eq!(provider.gaps(), [120, 600, 300]);
    let runtime = harness.core.model().runtime[&work_id()].clone();
    assert_eq!(runtime.failures, 0);
    assert_eq!(runtime.rate_limits, 0);
    assert_eq!(runtime.hold_until, None);
}

#[tokio::test(start_paused = true)]
async fn slow_providers_time_out_after_thirty_seconds() {
    let mut scripted = ScriptedProvider::new(Vec::new());
    scripted.stall = Some(Duration::from_secs(45));
    let provider = Arc::new(scripted);
    let (harness, _scheduler) = start(&provider).await;
    eventually_virtual(|| status(&harness) == AccountStatus::Error).await;
    let error = harness.core.state().accounts[0].error.clone().unwrap();
    assert_eq!(error.kind, "timeout");
}

#[tokio::test]
async fn success_persists_snapshot_and_identity_and_failure_keeps_it() {
    let provider = Arc::new(ScriptedProvider::new(vec![Ok(good())]));
    let (harness, _scheduler) = start(&provider).await;
    eventually(|| status(&harness) == AccountStatus::Fresh).await;
    let stored = harness
        .storage
        .blocking(|conn| snapshots::load_all(conn))
        .unwrap();
    assert_eq!(stored.len(), 1);
    let records = harness
        .storage
        .blocking(|conn| accounts::load_all(conn))
        .unwrap();
    assert_eq!(records[0].plan.as_deref(), Some("Pro"));
    provider
        .script
        .lock()
        .unwrap()
        .push_back(Err(ProviderError::Network("down".into())));
    harness.core.refresh("codex:work").unwrap();
    eventually(|| status(&harness) == AccountStatus::Error).await;
    let state = harness.core.state();
    assert_eq!(state.accounts[0].windows.len(), 1);
    assert_eq!(state.accounts[0].error.as_ref().unwrap().kind, "network");
}

fn lapsed() -> Outcome {
    Err(ProviderError::NoSubscription {
        detail: "No active ChatGPT subscription.".into(),
    })
}

#[tokio::test]
async fn lapsed_subscription_drops_data_notifies_once_and_rechecks_hourly() {
    let provider = Arc::new(ScriptedProvider::new(vec![Ok(good()), lapsed(), lapsed()]));
    let (harness, _scheduler) = start(&provider).await;
    eventually(|| status(&harness) == AccountStatus::Fresh).await;
    harness.core.refresh("codex:work").unwrap();
    eventually(|| status(&harness) == AccountStatus::NoSubscription).await;
    let state = harness.core.state();
    assert!(state.accounts[0].windows.is_empty());
    assert_eq!(state.headline, None);
    let recheck = state
        .generated_at
        .checked_add(SignedDuration::from_hours(1));
    assert_eq!(state.next_refresh_at, recheck.ok());
    let stored = harness.storage.blocking(|conn| snapshots::load_all(conn));
    assert!(stored.unwrap().is_empty());
    let expected = [(
        "Codex · ada@example.com — subscription inactive".to_owned(),
        "Limits are unavailable until the plan is renewed.".to_owned(),
    )];
    assert_eq!(harness.notifier.texts(), expected);
    harness.core.refresh("codex:work").unwrap();
    eventually(|| provider.calls() == 3 && status(&harness) == AccountStatus::NoSubscription).await;
    assert_eq!(harness.notifier.texts(), expected);
    harness.core.refresh("codex:work").unwrap();
    eventually(|| status(&harness) == AccountStatus::Fresh).await;
    let remaining = harness.storage.blocking(|conn| lapses::load_all(conn));
    assert!(remaining.unwrap().is_empty());
}

#[tokio::test]
async fn completed_refreshes_schedule_the_next_one_in_the_state() {
    let provider = Arc::new(ScriptedProvider::new(Vec::new()));
    let (harness, _scheduler) = start(&provider).await;
    eventually(|| provider.calls() == 1 && status(&harness) == AccountStatus::Fresh).await;
    let state = harness.core.state();
    let next = state
        .generated_at
        .checked_add(SignedDuration::from_mins(5))
        .unwrap();
    assert_eq!(state.next_refresh_at, Some(next));
    assert_eq!(state.last_success_at, Some(state.generated_at));
    assert!(!state.offline);
}

#[tokio::test]
async fn network_failures_mark_the_state_offline_until_a_success() {
    let down = Err(ProviderError::Network("down".into()));
    let provider = Arc::new(ScriptedProvider::new(vec![down]));
    let (harness, _scheduler) = start(&provider).await;
    eventually(|| status(&harness) == AccountStatus::Error).await;
    let state = harness.core.state();
    assert!(state.offline);
    let retry = state
        .generated_at
        .checked_add(SignedDuration::from_mins(1))
        .unwrap();
    assert_eq!(state.next_refresh_at, Some(retry));
    harness.core.refresh("codex:work").unwrap();
    eventually(|| status(&harness) == AccountStatus::Fresh).await;
    assert!(!harness.core.state().offline);
}

#[tokio::test]
async fn vanished_accounts_lose_their_worker() {
    let provider = Arc::new(ScriptedProvider::new(Vec::new()));
    let (harness, mut scheduler) = start(&provider).await;
    eventually(|| provider.calls() == 1 && status(&harness) == AccountStatus::Fresh).await;
    scheduler.sync(&[], FirstRefresh::Scheduled);
    harness.core.trigger(&work_id());
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert_eq!(provider.calls(), 1);
}

#[path = "adaptive_tests.rs"]
mod adaptive;
