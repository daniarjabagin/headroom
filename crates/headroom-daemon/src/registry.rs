use std::sync::Arc;
use std::time::Duration;

use headroom_core::provider::Provider;

use crate::core::Core;
use crate::rescan::{self, RescanRequests};
use crate::scheduler::{FirstRefresh, Scheduler};
use crate::storage::accounts;
use crate::usage::UsageWatchers;
use crate::usage::summary;

pub const DISCOVERY_EVERY: Duration = Duration::from_secs(600);
const DISCOVERY_TIMEOUT: Duration = Duration::from_secs(30);

struct Supervisor {
    core: Arc<Core>,
    scheduler: Scheduler,
    watchers: UsageWatchers,
}

pub async fn supervise(core: Arc<Core>, mut requests: RescanRequests) {
    let mut supervisor = Supervisor {
        scheduler: Scheduler::new(core.clone()),
        watchers: UsageWatchers::new(core.clone()),
        core,
    };
    let mut tick = tokio::time::interval(DISCOVERY_EVERY);
    loop {
        tokio::select! {
            biased;
            Some(waiters) = requests.next() => {
                supervisor.pass(FirstRefresh::Now).await;
                tick.reset();
                rescan::release(waiters);
            }
            _ = tick.tick() => supervisor.pass(FirstRefresh::Scheduled).await,
        }
    }
}

impl Supervisor {
    async fn pass(&mut self, first: FirstRefresh) {
        discover_all(&self.core).await;
        prune_usage(&self.core).await;
        self.scheduler.sync(&self.core.active_accounts(), first);
        let homes = self.core.model().usage_homes.clone();
        self.watchers.sync(&homes);
    }
}

pub async fn discover_all(core: &Core) {
    for provider in &core.providers {
        discover(core, provider.as_ref()).await;
        discover_usage_homes(core, provider.as_ref()).await;
    }
    if let Err(error) = core.reload_accounts().await {
        tracing::warn!(%error, "could not reload accounts");
    }
}

async fn discover(core: &Core, provider: &dyn Provider) {
    let kind = provider.kind();
    let found = match tokio::time::timeout(DISCOVERY_TIMEOUT, provider.discover()).await {
        Ok(Ok(found)) => found,
        Ok(Err(error)) => {
            tracing::warn!(provider = %kind, %error, "account discovery failed");
            return;
        }
        Err(_) => {
            tracing::warn!(provider = %kind, "account discovery timed out");
            return;
        }
    };
    let now = core.clock.now();
    let result = core
        .storage
        .run(move |conn| accounts::sync_provider(conn, kind, &found, now))
        .await;
    if let Err(error) = result {
        tracing::warn!(provider = %kind, %error, "could not store discovered accounts");
    }
}

async fn discover_usage_homes(core: &Core, provider: &dyn Provider) {
    let kind = provider.kind();
    match tokio::time::timeout(DISCOVERY_TIMEOUT, provider.usage_homes()).await {
        Ok(Ok(homes)) => core.set_usage_homes(kind, homes),
        Ok(Err(error)) => tracing::warn!(provider = %kind, %error, "usage home discovery failed"),
        Err(_) => tracing::warn!(provider = %kind, "usage home discovery timed out"),
    }
}

async fn prune_usage(core: &Core) {
    let now = core.clock.now();
    match core
        .storage
        .run(move |conn| summary::prune(conn, now))
        .await
    {
        Ok(0) => {}
        Ok(removed) => tracing::debug!(removed, "pruned old usage events"),
        Err(error) => tracing::warn!(%error, "could not prune usage events"),
    }
}

#[cfg(test)]
#[path = "registry_tests.rs"]
mod tests;
