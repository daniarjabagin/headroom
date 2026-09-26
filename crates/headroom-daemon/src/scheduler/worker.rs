use std::sync::Arc;
use std::time::Duration;

use headroom_core::account::AccountRef;
use jiff::SignedDuration;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio::time::{Instant, sleep_until};

use super::refresh::refresh_account;
use super::{FirstRefresh, policy};
use crate::core::Core;

pub struct Worker {
    pub account: AccountRef,
    task: JoinHandle<()>,
}

impl Worker {
    pub fn spawn(core: Arc<Core>, account: AccountRef, first: FirstRefresh) -> Worker {
        let (sender, receiver) = mpsc::channel(1);
        core.register_trigger(account.id.clone(), sender);
        let task = tokio::spawn(run(core, account.clone(), first, receiver));
        Worker { account, task }
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        self.task.abort();
    }
}

async fn run(
    core: Arc<Core>,
    account: AccountRef,
    first: FirstRefresh,
    mut triggers: mpsc::Receiver<()>,
) {
    let mut activity = core.activity_changes();
    let mut deadline = Instant::now() + to_std(first_delay(&core, &account, first));
    loop {
        tokio::select! {
            () = sleep_until(deadline) => {}
            received = triggers.recv() => {
                if received.is_none() {
                    return;
                }
            }
            Ok(()) = activity.changed() => {
                deadline = live_deadline(&core, &account, deadline);
                continue;
            }
        }
        let delay = refresh_account(&core, &account).await;
        deadline = Instant::now() + to_std(delay);
    }
}

fn first_delay(core: &Core, account: &AccountRef, first: FirstRefresh) -> SignedDuration {
    let now = core.clock.now();
    let mut model = core.model();
    let delay = match first {
        FirstRefresh::Now => SignedDuration::ZERO,
        FirstRefresh::Scheduled => {
            let fetched = model
                .snapshots
                .get(&account.id)
                .map(|e| e.snapshot.fetched_at);
            let live = model.account_is_live(account, now);
            let min_poll = core.catalog.min_poll_interval(&account.provider);
            let interval = policy::provider_interval(&model.settings, live, min_poll);
            policy::initial_delay(fetched, now, interval)
        }
    };
    model.runtime_mut(&account.id).next_refresh_at = now.checked_add(delay).ok();
    drop(model);
    core.mark_changed();
    delay
}

fn live_deadline(core: &Core, account: &AccountRef, deadline: Instant) -> Instant {
    let now = core.clock.now();
    let mut model = core.model();
    let live = model.account_is_live(account, now);
    if !policy::adaptive_live(&model.settings, live) {
        return deadline;
    }
    let min_poll = core.catalog.min_poll_interval(&account.provider);
    let interval = policy::provider_interval(&model.settings, live, min_poll);
    let Some(delay) = policy::live_delay(model.runtime.get(&account.id), interval, now) else {
        return deadline;
    };
    let sooner = Instant::now() + to_std(delay);
    if sooner >= deadline {
        return deadline;
    }
    model.runtime_mut(&account.id).next_refresh_at = now.checked_add(delay).ok();
    drop(model);
    core.mark_changed();
    sooner
}

fn to_std(delay: SignedDuration) -> Duration {
    Duration::try_from(delay).unwrap_or_default()
}
