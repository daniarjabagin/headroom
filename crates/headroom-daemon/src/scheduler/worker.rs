use std::sync::Arc;
use std::time::Duration;

use headroom_core::account::AccountRef;
use jiff::SignedDuration;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio::time::{Instant, sleep_until};

use super::policy;
use super::refresh::refresh_account;
use crate::core::Core;

pub struct Worker {
    pub account: AccountRef,
    task: JoinHandle<()>,
}

impl Worker {
    pub fn spawn(core: Arc<Core>, account: AccountRef) -> Worker {
        let (sender, receiver) = mpsc::channel(1);
        core.register_trigger(account.id.clone(), sender);
        let task = tokio::spawn(run(core, account.clone(), receiver));
        Worker { account, task }
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        self.task.abort();
    }
}

async fn run(core: Arc<Core>, account: AccountRef, mut triggers: mpsc::Receiver<()>) {
    let mut deadline = Instant::now() + to_std(first_delay(&core, &account));
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

fn first_delay(core: &Core, account: &AccountRef) -> SignedDuration {
    let model = core.model();
    let fetched = model
        .snapshots
        .get(&account.id)
        .map(|e| e.snapshot.fetched_at);
    policy::initial_delay(fetched, core.clock.now(), model.settings.refresh_interval())
}

fn to_std(delay: SignedDuration) -> Duration {
    Duration::try_from(delay).unwrap_or_default()
}
