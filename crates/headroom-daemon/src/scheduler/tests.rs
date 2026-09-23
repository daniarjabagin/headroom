use std::collections::VecDeque;
use std::path::Path;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use async_trait::async_trait;
use headroom_core::account::{AccountId, ProviderKind};
use headroom_core::cursor::LogCursors;
use headroom_core::event::UsageEvent;
use headroom_core::provider::{Provider, ProviderError};
use headroom_core::quota::LimitsSnapshot;
use jiff::SignedDuration;
use tokio::sync::Semaphore;
use tokio::time::Instant;

use super::*;
use crate::error::CommandError;
use crate::state::payload::AccountStatus;
use crate::storage::{accounts, snapshots};
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
            account: account(ProviderKind::Codex, "work"),
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
    fn kind(&self) -> ProviderKind {
        ProviderKind::Codex
    }

    async fn discover(&self) -> Result<Vec<AccountRef>, ProviderError> {
        Ok(vec![self.account.clone()])
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
    scheduler.sync(&harness.core.active_accounts());
    (harness, scheduler)
}

fn work_id() -> AccountId {
    account(ProviderKind::Codex, "work").id
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
async fn rate_limits_honour_retry_after_then_resume_the_interval() {
    let limited = Err(ProviderError::RateLimited {
        retry_after: Some(SignedDuration::from_secs(120)),
    });
    let unspecified = Err(ProviderError::RateLimited { retry_after: None });
    let provider = Arc::new(ScriptedProvider::new(vec![limited, unspecified]));
    let (harness, _scheduler) = start(&provider).await;
    eventually_virtual(|| provider.calls() == 4).await;
    assert_eq!(provider.gaps(), [120, 300, 300]);
    let runtime = harness.core.model().runtime[&work_id()].clone();
    assert_eq!(runtime.failures, 0);
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

#[tokio::test]
async fn vanished_accounts_lose_their_worker() {
    let provider = Arc::new(ScriptedProvider::new(Vec::new()));
    let (harness, mut scheduler) = start(&provider).await;
    eventually(|| provider.calls() == 1 && status(&harness) == AccountStatus::Fresh).await;
    scheduler.sync(&[]);
    harness.core.trigger(&work_id());
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert_eq!(provider.calls(), 1);
}
