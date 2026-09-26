use std::sync::Arc;

use headroom_core::provider::{Provider, ProviderError};
use jiff::Timestamp;
use jiff::civil::Date;

use super::summary::summarize;
use super::weighted::recent_spend;
use crate::activity;
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

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct IngestedLogs {
    pub changed: usize,
    pub latest: Option<Timestamp>,
}

pub async fn ingest(core: &Core, home: &UsageHome) -> Result<IngestedLogs, IngestError> {
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
) -> Result<IngestedLogs, IngestError> {
    let mut cursors = storage.blocking(|conn| cursors::load(conn, home))?;
    let events = provider.read_usage(&home.home, &mut cursors)?;
    let ingested = storage.blocking(|conn| events::ingest(conn, home, &events, &cursors))?;
    Ok(IngestedLogs {
        changed: ingested.changed,
        latest: events.iter().map(|event| event.at).max(),
    })
}

pub async fn refresh_summary(core: &Core, home: &UsageHome) -> Result<(), StorageError> {
    let now = core.clock.now();
    let prices = Arc::clone(&core.price_book);
    let tz = core.tz.clone();
    let key = home.clone();
    let (summary, spend) = core
        .storage
        .run(move |conn| {
            let summary = summarize(conn, &key, prices.as_ref(), &tz, now)?;
            Ok((summary, recent_spend(conn, &key, prices.as_ref(), now)?))
        })
        .await?;
    let mut model = core.model();
    model.usage.insert(home.clone(), summary);
    model.history.set_spend(home, spend);
    drop(model);
    core.mark_changed();
    Ok(())
}

pub async fn pass(core: &Core, home: &UsageHome, summarized_for: &mut Option<Date>) {
    let changed = ingest_and_track(core, home).await;
    let today = core.tz.to_datetime(core.clock.now()).date();
    if changed == 0 && *summarized_for == Some(today) {
        return;
    }
    match refresh_summary(core, home).await {
        Ok(()) => *summarized_for = Some(today),
        Err(error) => tracing::warn!(%error, "usage summary failed"),
    }
}

async fn ingest_and_track(core: &Core, home: &UsageHome) -> usize {
    match ingest(core, home).await {
        Ok(ingested) => {
            if ingested.changed > 0 {
                activity::record_write(core, home, ingested.latest);
            }
            ingested.changed
        }
        Err(error) => {
            tracing::warn!(provider = %home.provider, home = %home.home.display(), %error, "usage ingest failed");
            0
        }
    }
}
