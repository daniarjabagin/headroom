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
    let mut deadline = Instant::now() + to_std(first_delay(&core, &account, first));
    loop {
        tokio::select! {
            () = sleep_until(deadline) => {}
            received = triggers.recv() => {
                if received.is_none() {
                    return;
                }
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
            policy::initial_delay(fetched, now, model.settings.refresh_interval())
        }
    };
    model.runtime_mut(&account.id).next_refresh_at = now.checked_add(delay).ok();
    drop(model);
    core.mark_changed();
    delay
}

fn to_std(delay: SignedDuration) -> Duration {
    Duration::try_from(delay).unwrap_or_default()
}
