use std::process::ExitCode;
use std::sync::Arc;

use anyhow::{Context, Result};
use headroom_core::provider::Provider;
use headroom_daemon::{DaemonConfig, DaemonError, Shutdown};
use headroom_providers::claude::{ClaudeConfig, ClaudeProvider};
use headroom_providers::codex::{CodexConfig, CodexProvider};
use tokio::signal::unix::{SignalKind, signal};

use crate::paths::{Globals, pricing_cache_dir};
use crate::pricing::{ReloadablePrices, keep_fresh};

pub async fn run(globals: &Globals) -> Result<ExitCode> {
    let prices = Arc::new(ReloadablePrices::load(pricing_cache_dir()?)?);
    let http = headroom_providers::http::client().context("cannot create the HTTP client")?;
    let mut config = DaemonConfig::new(providers(&http), prices.clone(), shutdown_signal()?)?;
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

fn providers(http: &reqwest::Client) -> Vec<Arc<dyn Provider>> {
    let codex = CodexProvider::with_http(CodexConfig::from_process(), http.clone());
    let mut providers: Vec<Arc<dyn Provider>> = vec![Arc::new(codex)];
    match ClaudeConfig::from_env() {
        Ok(config) => providers.push(Arc::new(ClaudeProvider::with_http(config, http.clone()))),
        Err(error) => tracing::error!(%error, "Claude provider unavailable"),
    }
    providers
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
