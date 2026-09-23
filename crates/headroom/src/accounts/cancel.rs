use std::io;

use anyhow::{Context, Result};
use tokio::io::Interest;
use tokio::io::unix::AsyncFd;
use tokio::signal::unix::{SignalKind, signal};
use tokio::sync::watch;

pub const CANCELLED: &str = "cancelled";

#[derive(Debug, Clone)]
pub struct Cancel {
    state: watch::Sender<bool>,
}

impl Default for Cancel {
    fn default() -> Cancel {
        Cancel {
            state: watch::Sender::new(false),
        }
    }
}

impl Cancel {
    pub fn cancel(&self) {
        self.state.send_replace(true);
    }

    pub fn is_cancelled(&self) -> bool {
        *self.state.borrow()
    }

    pub async fn cancelled(&self) {
        let mut state = self.state.subscribe();
        if state.wait_for(|cancelled| *cancelled).await.is_err() {
            std::future::pending::<()>().await;
        }
    }

    pub fn on_signals(&self) -> Result<()> {
        let mut terminate = signal(SignalKind::terminate()).context("could not watch SIGTERM")?;
        let mut interrupt = signal(SignalKind::interrupt()).context("could not watch SIGINT")?;
        let cancel = self.clone();
        tokio::spawn(async move {
            tokio::select! {
                _ = terminate.recv() => {}
                _ = interrupt.recv() => {}
            }
            cancel.cancel();
        });
        Ok(())
    }

    pub fn on_stdout_closed(&self) {
        let Ok(stdout) = AsyncFd::with_interest(io::stdout(), Interest::ERROR) else {
            return;
        };
        let cancel = self.clone();
        tokio::spawn(async move {
            if stdout.ready(Interest::ERROR).await.is_ok() {
                cancel.cancel();
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    #[tokio::test]
    async fn waiters_wake_when_cancelled_from_another_thread() {
        let cancel = Cancel::default();
        assert!(!cancel.is_cancelled());
        let remote = cancel.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(20));
            remote.cancel();
        });
        tokio::time::timeout(Duration::from_secs(5), cancel.cancelled())
            .await
            .unwrap();
        assert!(cancel.is_cancelled());
        cancel.cancelled().await;
    }
}
