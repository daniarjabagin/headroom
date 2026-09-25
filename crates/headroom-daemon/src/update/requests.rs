use tokio::sync::{mpsc, oneshot};

use super::outcome::CheckOutcome;
use crate::error::CommandError;

const QUEUE: usize = 32;

type Waiter = oneshot::Sender<CheckOutcome>;

#[derive(Debug, Clone)]
pub struct UpdateChecks {
    sender: mpsc::Sender<Waiter>,
}

#[derive(Debug)]
pub struct UpdateCheckRequests {
    receiver: mpsc::Receiver<Waiter>,
}

#[must_use]
pub fn channel() -> (UpdateChecks, UpdateCheckRequests) {
    let (sender, receiver) = mpsc::channel(QUEUE);
    (UpdateChecks { sender }, UpdateCheckRequests { receiver })
}

impl UpdateChecks {
    pub async fn check(&self) -> Result<CheckOutcome, CommandError> {
        let (waiter, done) = oneshot::channel();
        self.sender
            .send(waiter)
            .await
            .map_err(|_| CommandError::Stopping)?;
        done.await.map_err(|_| CommandError::Stopping)
    }
}

impl UpdateCheckRequests {
    pub async fn next(&mut self) -> Option<Vec<Waiter>> {
        let first = self.receiver.recv().await?;
        let mut waiters = vec![first];
        while let Ok(waiter) = self.receiver.try_recv() {
            waiters.push(waiter);
        }
        Some(waiters)
    }
}

pub fn answer(waiters: Vec<Waiter>, outcome: &CheckOutcome) {
    for waiter in waiters {
        waiter.send(outcome.clone()).ok();
    }
}
