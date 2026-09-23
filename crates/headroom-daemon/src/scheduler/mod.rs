pub mod policy;
mod refresh;
mod worker;

use std::collections::HashMap;
use std::sync::Arc;

use headroom_core::account::{AccountId, AccountRef};

use crate::core::Core;
use worker::Worker;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FirstRefresh {
    Scheduled,
    Now,
}

pub struct Scheduler {
    core: Arc<Core>,
    workers: HashMap<AccountId, Worker>,
}

impl Scheduler {
    #[must_use]
    pub fn new(core: Arc<Core>) -> Scheduler {
        Scheduler {
            core,
            workers: HashMap::new(),
        }
    }

    pub fn sync(&mut self, accounts: &[AccountRef], first: FirstRefresh) {
        self.workers.retain(|id, worker| {
            let keep = accounts.contains(&worker.account);
            if !keep {
                self.core.remove_trigger(id);
            }
            keep
        });
        for account in accounts {
            if !self.workers.contains_key(&account.id) {
                let worker = Worker::spawn(self.core.clone(), account.clone(), first);
                self.workers.insert(account.id.clone(), worker);
            }
        }
    }
}

#[cfg(test)]
mod tests;
