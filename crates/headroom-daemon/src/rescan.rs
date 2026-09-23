use tokio::sync::{mpsc, oneshot};

use crate::error::CommandError;

const QUEUE: usize = 32;

type Waiter = oneshot::Sender<()>;

#[derive(Debug, Clone)]
pub struct Rescans {
    sender: mpsc::Sender<Waiter>,
}

#[derive(Debug)]
pub struct RescanRequests {
    receiver: mpsc::Receiver<Waiter>,
}

#[must_use]
pub fn channel() -> (Rescans, RescanRequests) {
    let (sender, receiver) = mpsc::channel(QUEUE);
    (Rescans { sender }, RescanRequests { receiver })
}

impl Rescans {
    pub async fn rescan(&self) -> Result<(), CommandError> {
        let (waiter, done) = oneshot::channel();
        self.sender
            .send(waiter)
            .await
            .map_err(|_| CommandError::Stopping)?;
        done.await.map_err(|_| CommandError::Stopping)
    }
}

impl RescanRequests {
    pub async fn next(&mut self) -> Option<Vec<Waiter>> {
        let first = self.receiver.recv().await?;
        let mut waiters = vec![first];
        while let Ok(waiter) = self.receiver.try_recv() {
            waiters.push(waiter);
        }
        Some(waiters)
    }
}

pub fn release(waiters: Vec<Waiter>) {
    for waiter in waiters {
        waiter.send(()).ok();
    }
}
