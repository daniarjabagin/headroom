use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use async_trait::async_trait;
use tokio::sync::mpsc::{self, error::TrySendError};

use super::protocol::{alert_line, open_requested_line, state_changed_line};
use crate::events::{EventError, EventSink};
use crate::notify::{Notification, Notifier, NotifyError};

pub type Outbox = mpsc::Sender<Arc<str>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Topic {
    State,
    Alerts,
    Open,
}

impl Topic {
    #[must_use]
    pub fn parse(name: &str) -> Option<Topic> {
        match name {
            "state" => Some(Topic::State),
            "alerts" => Some(Topic::Alerts),
            "open" => Some(Topic::Open),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Topics {
    state: bool,
    alerts: bool,
    open: bool,
}

impl Topics {
    pub const ALL: Topics = Topics {
        state: true,
        alerts: true,
        open: true,
    };

    /// The listed topics; an empty list means every topic.
    #[must_use]
    pub fn from_list(topics: &[Topic]) -> Topics {
        if topics.is_empty() {
            return Topics::ALL;
        }
        Topics {
            state: topics.contains(&Topic::State),
            alerts: topics.contains(&Topic::Alerts),
            open: topics.contains(&Topic::Open),
        }
    }

    #[must_use]
    pub fn contains(self, topic: Topic) -> bool {
        match topic {
            Topic::State => self.state,
            Topic::Alerts => self.alerts,
            Topic::Open => self.open,
        }
    }
}

struct Subscriber {
    outbox: Outbox,
    topics: Topics,
}

#[derive(Default)]
pub struct Hub {
    subscribers: Mutex<HashMap<u64, Subscriber>>,
    next_id: AtomicU64,
}

pub struct Subscription {
    hub: Arc<Hub>,
    id: u64,
}

impl Hub {
    #[must_use]
    pub fn subscribe(self: &Arc<Self>, outbox: Outbox, topics: Topics) -> Subscription {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        self.subscribers().insert(id, Subscriber { outbox, topics });
        Subscription {
            hub: self.clone(),
            id,
        }
    }

    pub fn broadcast(&self, topic: Topic, line: &str) -> usize {
        let line: Arc<str> = Arc::from(line);
        let mut subscribers = self.subscribers();
        let mut delivered = 0;
        subscribers.retain(|id, subscriber| {
            if !subscriber.topics.contains(topic) {
                return true;
            }
            match subscriber.outbox.try_send(line.clone()) {
                Ok(()) => {
                    delivered += 1;
                    true
                }
                Err(TrySendError::Full(_)) => {
                    tracing::warn!(subscriber = id, "socket subscriber is not reading, skipped");
                    true
                }
                Err(TrySendError::Closed(_)) => false,
            }
        });
        delivered
    }

    fn subscribers(&self) -> MutexGuard<'_, HashMap<u64, Subscriber>> {
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
        self.broadcast(Topic::State, &state_changed_line(state));
        Ok(())
    }

    async fn open_requested(&self) -> Result<(), EventError> {
        self.broadcast(Topic::Open, &open_requested_line());
        Ok(())
    }
}

#[async_trait]
impl Notifier for Hub {
    async fn notify(&self, notification: &Notification) -> Result<(), NotifyError> {
        match self.broadcast(Topic::Alerts, &alert_line(notification)) {
            0 => Err(NotifyError::NoSubscribers),
            _ => Ok(()),
        }
    }
}
