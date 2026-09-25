mod targets;
mod watcher;

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

use headroom_core::account::AccountId;
use tokio::task::JoinHandle;
use tokio::time::MissedTickBehavior;

use crate::service::Service;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WatchTiming {
    pub sync_every: Duration,
    pub settle: Duration,
}

pub const TIMING: WatchTiming = WatchTiming {
    sync_every: Duration::from_secs(5),
    settle: Duration::from_secs(2),
};

struct FileWatch {
    path: PathBuf,
    task: JoinHandle<()>,
}

impl Drop for FileWatch {
    fn drop(&mut self) {
        self.task.abort();
    }
}

pub async fn watch_signed_out(service: Service, timing: WatchTiming) {
    let mut watches = HashMap::new();
    let mut tick = tokio::time::interval(timing.sync_every);
    tick.set_missed_tick_behavior(MissedTickBehavior::Delay);
    loop {
        tick.tick().await;
        sync(&service, &mut watches, timing.settle);
    }
}

fn sync(service: &Service, watches: &mut HashMap<AccountId, FileWatch>, settle: Duration) {
    let wanted = {
        let core = service.core();
        targets::credential_files(&core.model(), &core.catalog)
    };
    watches.retain(|id, watch| wanted.get(id) == Some(&watch.path) && !watch.task.is_finished());
    for (id, path) in wanted {
        if watches.contains_key(&id) {
            continue;
        }
        let task = tokio::spawn(watcher::watch_file(
            service.clone(),
            id.clone(),
            path.clone(),
            settle,
        ));
        watches.insert(id, FileWatch { path, task });
    }
}

#[cfg(test)]
mod tests;
