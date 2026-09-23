pub mod alerts;
pub mod desktop;
pub mod evaluator;
pub mod text;

use async_trait::async_trait;

pub use text::Notification;

#[derive(Debug, thiserror::Error)]
pub enum NotifyError {
    #[error("notification delivery failed: {0}")]
    Bus(#[from] zbus::Error),
}

#[async_trait]
pub trait Notifier: Send + Sync {
    async fn notify(&self, notification: &Notification) -> Result<(), NotifyError>;
}
