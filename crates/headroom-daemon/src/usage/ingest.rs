use std::sync::Arc;

use headroom_core::provider::{Provider, ProviderError};
use jiff::civil::Date;

use super::summary::summarize;
use crate::core::Core;
use crate::error::StorageError;
use crate::home::UsageHome;
use crate::storage::{Storage, cursors, events};

#[derive(Debug, thiserror::Error)]
pub enum IngestError {
    #[error("no provider is registered for {0}")]
    NoProvider(String),
    #[error(transparent)]
    Provider(#[from] ProviderError),
    #[error(transparent)]
    Storage(#[from] StorageError),
    #[error("usage task failed: {0}")]
    Task(String),
}

pub async fn ingest(core: &Core, home: &UsageHome) -> Result<usize, IngestError> {
    let provider = core
        .provider(&home.provider)
        .ok_or_else(|| IngestError::NoProvider(home.provider.to_string()))?;
    let storage = core.storage.clone();
    let home = home.clone();
    let _one_home_at_a_time = core.log_reads.lock().await;
    tokio::task::spawn_blocking(move || ingest_blocking(&storage, provider.as_ref(), &home))
        .await
        .map_err(|error| IngestError::Task(error.to_string()))?
}

fn ingest_blocking(
    storage: &Storage,
    provider: &dyn Provider,
    home: &UsageHome,
) -> Result<usize, IngestError> {
    let mut cursors = storage.blocking(|conn| cursors::load(conn, home))?;
    let events = provider.read_usage(&home.home, &mut cursors)?;
    let ingested = storage.blocking(|conn| events::ingest(conn, home, &events, &cursors))?;
    Ok(ingested.changed)
}

pub async fn refresh_summary(core: &Core, home: &UsageHome) -> Result<(), StorageError> {
    let now = core.clock.now();
    let prices = Arc::clone(&core.price_book);
    let tz = core.tz.clone();
    let key = home.clone();
    let summary = core
        .storage
        .run(move |conn| summarize(conn, &key, prices.as_ref(), &tz, now))
        .await?;
    core.model().usage.insert(home.clone(), summary);
    core.mark_changed();
    Ok(())
}

pub async fn pass(core: &Core, home: &UsageHome, summarized_for: &mut Option<Date>) {
    let changed = match ingest(core, home).await {
        Ok(changed) => changed,
        Err(error) => {
            tracing::warn!(provider = %home.provider, home = %home.home.display(), %error, "usage ingest failed");
            0
        }
    };
    let today = core.tz.to_datetime(core.clock.now()).date();
    if changed == 0 && *summarized_for == Some(today) {
        return;
    }
    match refresh_summary(core, home).await {
        Ok(()) => *summarized_for = Some(today),
        Err(error) => tracing::warn!(%error, "usage summary failed"),
    }
}
