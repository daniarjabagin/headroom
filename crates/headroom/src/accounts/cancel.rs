use std::io;
use std::os::fd::AsRawFd;

use anyhow::{Context, Result};
use tokio::io::unix::AsyncFd;
use tokio::io::{Interest, Ready};
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
        let Ok(stdout) = AsyncFd::with_interest(io::stdout(), Interest::WRITABLE) else {
            return;
        };
        let cancel = self.clone();
        tokio::spawn(async move {
            if reader_gone(&stdout).await.is_ok() {
                cancel.cancel();
            }
        });
    }
}

async fn reader_gone<T: AsRawFd>(writer: &AsyncFd<T>) -> io::Result<()> {
    loop {
        let mut guard = writer.ready(Interest::WRITABLE).await?;
        if signals_reader_gone(guard.ready()) {
            return Ok(());
        }
        guard.clear_ready();
    }
}

fn signals_reader_gone(ready: Ready) -> bool {
    ready.is_write_closed() || ready.is_error()
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

    #[test]
    fn only_closed_or_failed_writers_mean_the_reader_is_gone() {
        let cases = [
            (Ready::EMPTY, false),
            (Ready::WRITABLE, false),
            (Ready::READABLE, false),
            (Ready::WRITE_CLOSED, true),
            (Ready::WRITABLE | Ready::WRITE_CLOSED, true),
            (Ready::ERROR, true),
            (Ready::WRITABLE | Ready::ERROR, true),
        ];
        for (ready, expected) in cases {
            assert_eq!(signals_reader_gone(ready), expected, "{ready:?}");
        }
    }

    #[tokio::test]
    async fn an_idle_writable_pipe_keeps_waiting() {
        let (_reader, writer) = io::pipe().unwrap();
        let writer = AsyncFd::with_interest(writer, Interest::WRITABLE).unwrap();
        let waited = tokio::time::timeout(Duration::from_millis(200), reader_gone(&writer)).await;
        assert!(waited.is_err());
    }

    #[tokio::test]
    async fn closing_the_reader_is_noticed() {
        let (reader, writer) = io::pipe().unwrap();
        let writer = AsyncFd::with_interest(writer, Interest::WRITABLE).unwrap();
        let watch = tokio::spawn(async move { reader_gone(&writer).await });
        tokio::time::sleep(Duration::from_millis(50)).await;
        drop(reader);
        tokio::time::timeout(Duration::from_secs(5), watch)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
    }

    #[tokio::test]
    async fn a_reader_closed_before_watching_is_noticed() {
        let (reader, writer) = io::pipe().unwrap();
        drop(reader);
        let writer = AsyncFd::with_interest(writer, Interest::WRITABLE).unwrap();
        tokio::time::timeout(Duration::from_secs(5), reader_gone(&writer))
            .await
            .unwrap()
            .unwrap();
    }
}
