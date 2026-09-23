use std::sync::Arc;
use std::time::Duration;

use headroom_core::provider::Provider;

use crate::core::Core;
use crate::scheduler::Scheduler;
use crate::storage::accounts;
use crate::usage::UsageWatchers;
use crate::usage::summary;

pub const DISCOVERY_EVERY: Duration = Duration::from_secs(600);
const DISCOVERY_TIMEOUT: Duration = Duration::from_secs(30);

pub async fn supervise(core: Arc<Core>) {
    let mut scheduler = Scheduler::new(core.clone());
    let mut watchers = UsageWatchers::new(core.clone());
    let mut tick = tokio::time::interval(DISCOVERY_EVERY);
    loop {
        tick.tick().await;
        discover_all(&core).await;
        prune_usage(&core).await;
        scheduler.sync(&core.active_accounts());
        let homes = core.model().usage_homes();
        watchers.sync(&homes);
    }
}

pub async fn discover_all(core: &Core) {
    for provider in &core.providers {
        discover(core, provider.as_ref()).await;
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
