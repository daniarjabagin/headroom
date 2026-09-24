use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;

use crate::core::Core;

pub const DEBOUNCE: Duration = Duration::from_millis(250);

#[derive(Debug, thiserror::Error)]
pub enum EventError {
    #[cfg(target_os = "linux")]
    #[error("D-Bus signal failed: {0}")]
    Bus(#[from] zbus::Error),
}

#[async_trait]
pub trait EventSink: Send + Sync {
    async fn state_changed(&self, state: &str) -> Result<(), EventError>;
    async fn open_requested(&self) -> Result<(), EventError>;
}

#[derive(Default)]
pub struct EventSinks {
    sinks: Vec<Arc<dyn EventSink>>,
}

impl EventSinks {
    pub fn push(&mut self, sink: Arc<dyn EventSink>) {
        self.sinks.push(sink);
    }
}

#[async_trait]
impl EventSink for EventSinks {
    async fn state_changed(&self, state: &str) -> Result<(), EventError> {
        let mut outcome = Ok(());
        for sink in &self.sinks {
            outcome = outcome.and(sink.state_changed(state).await);
        }
        outcome
    }

    async fn open_requested(&self) -> Result<(), EventError> {
        let mut outcome = Ok(());
        for sink in &self.sinks {
            outcome = outcome.and(sink.open_requested().await);
        }
        outcome
    }
}

pub async fn publish_changes(core: Arc<Core>, sink: Arc<dyn EventSink>) {
    loop {
        core.changed().await;
        tokio::time::sleep(DEBOUNCE).await;
        match core.state_json() {
            Ok(state) => {
                if let Err(error) = sink.state_changed(&state).await {
                    tracing::warn!(%error, "could not emit StateChanged");
                }
            }
            Err(error) => tracing::error!(%error, "could not encode state"),
        }
    }
}
