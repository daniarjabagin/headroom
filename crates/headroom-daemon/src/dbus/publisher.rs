use std::sync::Arc;
use std::time::Duration;

use super::signals::SignalSink;
use crate::core::Core;

pub const DEBOUNCE: Duration = Duration::from_millis(250);

pub async fn publish_changes(core: Arc<Core>, sink: Arc<dyn SignalSink>) {
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
