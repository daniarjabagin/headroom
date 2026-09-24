pub mod alerts;
#[cfg(target_os = "linux")]
pub mod desktop;
pub mod evaluator;
pub mod text;

use async_trait::async_trait;

pub use text::{Notification, Urgency};

#[derive(Debug, thiserror::Error)]
pub enum NotifyError {
    #[cfg(target_os = "linux")]
    #[error("notification delivery failed: {0}")]
    Bus(#[from] zbus::Error),
    #[error("no app is subscribed to alerts")]
    NoSubscribers,
}

#[async_trait]
pub trait Notifier: Send + Sync {
    async fn notify(&self, notification: &Notification) -> Result<(), NotifyError>;
}
