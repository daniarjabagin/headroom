use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use async_trait::async_trait;
use tokio::sync::mpsc::{self, error::TrySendError};

use super::protocol::{alert_line, open_requested_line, state_changed_line};
use crate::events::{EventError, EventSink};
use crate::notify::{Notification, Notifier, NotifyError};

pub type Outbox = mpsc::Sender<Arc<str>>;

#[derive(Default)]
pub struct Hub {
    subscribers: Mutex<HashMap<u64, Outbox>>,
    next_id: AtomicU64,
}

pub struct Subscription {
    hub: Arc<Hub>,
    id: u64,
}

impl Hub {
    #[must_use]
    pub fn subscribe(self: &Arc<Self>, outbox: Outbox) -> Subscription {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        self.subscribers().insert(id, outbox);
        Subscription {
            hub: self.clone(),
            id,
        }
    }

    pub fn broadcast(&self, line: &str) -> usize {
        let line: Arc<str> = Arc::from(line);
        let mut subscribers = self.subscribers();
        let mut delivered = 0;
        subscribers.retain(|id, outbox| match outbox.try_send(line.clone()) {
            Ok(()) => {
                delivered += 1;
                true
            }
            Err(TrySendError::Full(_)) => {
                tracing::warn!(subscriber = id, "socket subscriber is not reading, skipped");
                true
            }
            Err(TrySendError::Closed(_)) => false,
        });
        delivered
    }

    fn subscribers(&self) -> MutexGuard<'_, HashMap<u64, Outbox>> {
        self.subscribers
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }
}

impl Drop for Subscription {
    fn drop(&mut self) {
        self.hub.subscribers().remove(&self.id);
    }
}

#[async_trait]
impl EventSink for Hub {
    async fn state_changed(&self, state: &str) -> Result<(), EventError> {
        self.broadcast(&state_changed_line(state));
        Ok(())
    }

    async fn open_requested(&self) -> Result<(), EventError> {
        self.broadcast(&open_requested_line());
        Ok(())
    }
}

#[async_trait]
impl Notifier for Hub {
    async fn notify(&self, notification: &Notification) -> Result<(), NotifyError> {
        match self.broadcast(&alert_line(notification)) {
            0 => Err(NotifyError::NoSubscribers),
            _ => Ok(()),
        }
    }
}
