use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;

use anyhow::{Context, Result};
use headroom_daemon::{DaemonConfig, DaemonError, LogControl, Shutdown, SocketError};
use tokio::signal::unix::{SignalKind, signal};

use crate::cli::DaemonArgs;
use crate::logging::DaemonLogging;
use crate::paths::{Globals, pricing_cache_dir};
use crate::pricing::{ReloadablePrices, keep_fresh};
use crate::providers::{self, LocalRegistry};
use crate::{status_pages, update};

pub async fn run(
    globals: &Globals,
    args: DaemonArgs,
    logging: Option<Arc<DaemonLogging>>,
) -> Result<ExitCode> {
    let prices = Arc::new(ReloadablePrices::load(pricing_cache_dir()?)?);
    let http = headroom_providers::http::client().context("cannot create the HTTP client")?;
    let registry = LocalRegistry::new(globals, http.clone())?;
    let mut config = DaemonConfig::new(registry.all(), prices.clone(), shutdown_signal()?)?;
    config.catalog = providers::catalog();
    config.db_path = globals.db_path()?;
    config.logging = logging.map(|logging| logging as Arc<dyn LogControl>);
    #[cfg(target_os = "linux")]
    {
        config.bus = globals.bus.clone();
    }
    if let Some(socket) = args.socket {
        config.socket = Some(socket_path(socket)?);
    }
    if !args.no_update_check {
        config.updates = Some(update::daemon_config()?);
    }
    config.status_pages = Some(Arc::new(status_pages::StatusPageClient::new()?));
    let pricing = tokio::spawn(keep_fresh(prices, http));
    let result = headroom_daemon::run(config).await;
    pricing.abort();
    let _ = pricing.await;
    match result {
        Ok(()) => Ok(ExitCode::SUCCESS),
        Err(DaemonError::AlreadyRunning) => {
            tracing::error!("another Headroom daemon is already running on this bus");
            Ok(ExitCode::FAILURE)
        }
        Err(DaemonError::Socket(error @ SocketError::AlreadyListening(_))) => {
            tracing::error!("{error}");
            Ok(ExitCode::FAILURE)
        }
        Err(error) => Err(error).context("the Headroom daemon stopped"),
    }
}

fn socket_path(requested: Option<PathBuf>) -> Result<PathBuf> {
    match requested {
        Some(path) => Ok(path),
        None => Ok(headroom_daemon::default_socket_path()?),
    }
}

fn shutdown_signal() -> Result<Shutdown> {
    let mut terminate = signal(SignalKind::terminate()).context("cannot listen for SIGTERM")?;
    let mut interrupt = signal(SignalKind::interrupt()).context("cannot listen for SIGINT")?;
    Ok(Box::pin(async move {
        tokio::select! {
            _ = terminate.recv() => tracing::info!("received SIGTERM"),
            _ = interrupt.recv() => tracing::info!("received SIGINT"),
        }
    }))
}
