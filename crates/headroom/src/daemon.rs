use std::process::ExitCode;
use std::sync::Arc;

use anyhow::{Context, Result};
use headroom_daemon::{DaemonConfig, DaemonError, Shutdown};
use tokio::signal::unix::{SignalKind, signal};

use crate::paths::{Globals, pricing_cache_dir};
use crate::pricing::{ReloadablePrices, keep_fresh};
use crate::providers::{self, LocalRegistry};

pub async fn run(globals: &Globals) -> Result<ExitCode> {
    let prices = Arc::new(ReloadablePrices::load(pricing_cache_dir()?)?);
    let http = headroom_providers::http::client().context("cannot create the HTTP client")?;
    let registry = LocalRegistry::new(globals, http.clone())?;
    let mut config = DaemonConfig::new(registry.all(), prices.clone(), shutdown_signal()?)?;
    config.catalog = providers::catalog();
    config.db_path = globals.db_path()?;
    config.bus = globals.bus.clone();
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
        Err(error) => Err(error).context("the Headroom daemon stopped"),
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
