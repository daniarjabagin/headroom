pub mod breakdown;
pub mod ingest;
pub mod report;
pub mod share;
pub mod summary;
mod watcher;

pub(crate) use watcher::is_relevant;

use std::collections::{BTreeSet, HashMap};
use std::sync::Arc;

use tokio::task::JoinHandle;

use crate::core::Core;
use crate::home::UsageHome;

pub struct UsageWatchers {
    core: Arc<Core>,
    tasks: HashMap<UsageHome, JoinHandle<()>>,
}

impl UsageWatchers {
    #[must_use]
    pub fn new(core: Arc<Core>) -> UsageWatchers {
        UsageWatchers {
            core,
            tasks: HashMap::new(),
        }
    }

    pub fn sync(&mut self, homes: &BTreeSet<UsageHome>) {
        self.tasks.retain(|home, task| {
            let keep = homes.contains(home);
            if !keep {
                task.abort();
            }
            keep
        });
        for home in homes {
            if !self.tasks.contains_key(home) {
                let task = tokio::spawn(watcher::watch_home(self.core.clone(), home.clone()));
                self.tasks.insert(home.clone(), task);
            }
        }
    }
}

impl Drop for UsageWatchers {
    fn drop(&mut self) {
        for task in self.tasks.values() {
            task.abort();
        }
    }
}

#[cfg(test)]
mod tests;
