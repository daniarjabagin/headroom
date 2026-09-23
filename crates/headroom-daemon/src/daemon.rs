use std::path::PathBuf;
use std::sync::Arc;

use tokio::task::JoinSet;

use crate::config::DaemonConfig;
use crate::core::{Core, CoreParts};
use crate::dbus::signals::{BusSignals, SignalSink};
use crate::dbus::{self, publisher};
use crate::error::{DaemonError, StorageError};
use crate::home::HomeDisplay;
use crate::notify::desktop::DesktopNotifier;
use crate::random::ThreadRandom;
use crate::registry;
use crate::storage::Storage;

pub async fn run(config: DaemonConfig) -> Result<(), DaemonError> {
    let storage = open_storage(config.db_path).await?;
    let conn = dbus::connect(&config.bus).await?;
    let notifier = Arc::new(DesktopNotifier::new(&conn).await?);
    let parts = CoreParts {
        storage,
        providers: config.providers,
        price_book: config.price_book,
        clock: config.clock,
        random: Arc::new(ThreadRandom),
        tz: config.tz,
        homes: HomeDisplay::new(dirs::home_dir()),
        notifier: notifier.clone(),
    };
    let core = Arc::new(Core::load(parts).await?);
    dbus::serve(&conn, core.clone()).await?;
    let sink: Arc<dyn SignalSink> = Arc::new(BusSignals::new(conn.clone()));
    let mut tasks = JoinSet::new();
    tasks.spawn(registry::supervise(core.clone()));
    tasks.spawn(publisher::publish_changes(core.clone(), sink.clone()));
    tasks.spawn(forward_actions(notifier, sink));
    tracing::info!(name = dbus::BUS_NAME, "headroom daemon running");
    config.shutdown.await;
    tasks.shutdown().await;
    tracing::info!("headroom daemon stopped");
    Ok(())
}

async fn open_storage(path: PathBuf) -> Result<Storage, DaemonError> {
    let opened = tokio::task::spawn_blocking(move || Storage::open(&path))
        .await
        .map_err(|error| StorageError::Task(error.to_string()))?;
    Ok(opened?)
}

async fn forward_actions(notifier: Arc<DesktopNotifier>, sink: Arc<dyn SignalSink>) {
    if let Err(error) = notifier.forward_actions(sink).await {
        tracing::warn!(%error, "stopped listening for notification actions");
    }
}
