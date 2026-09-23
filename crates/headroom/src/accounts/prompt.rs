use std::io::{self, Write};
use std::os::fd::{AsFd, AsRawFd};

use anyhow::{Context, Result, bail};
use rustix::termios::{self, LocalModes, OptionalActions, Termios};
use tokio::io::Interest;
use tokio::io::unix::AsyncFd;

use super::cancel::{CANCELLED, Cancel};

const PROMPT: &str = "Paste the API key: ";
const CHUNK: usize = 4096;

struct EchoOff<'a, T: AsFd> {
    tty: &'a T,
    saved: Termios,
}

impl<'a, T: AsFd> EchoOff<'a, T> {
    fn new(tty: &'a T) -> io::Result<EchoOff<'a, T>> {
        let saved = termios::tcgetattr(tty)?;
        let mut quiet = saved.clone();
        quiet.local_modes.remove(LocalModes::ECHO);
        quiet.local_modes.insert(LocalModes::ECHONL);
        termios::tcsetattr(tty, OptionalActions::Now, &quiet)?;
        Ok(EchoOff { tty, saved })
    }
}

impl<T: AsFd> Drop for EchoOff<'_, T> {
    fn drop(&mut self) {
        if let Err(error) = termios::tcsetattr(self.tty, OptionalActions::Now, &self.saved) {
            tracing::warn!(%error, "could not restore the terminal");
        }
    }
}

pub async fn read_hidden<T: AsFd + AsRawFd>(
    tty: T,
    out: &mut dyn Write,
    cancel: &Cancel,
) -> Result<String> {
    write!(out, "{PROMPT}")?;
    out.flush()?;
    let tty =
        AsyncFd::with_interest(tty, Interest::READABLE).context("could not watch the terminal")?;
    let echo = EchoOff::new(tty.get_ref()).context("could not turn off terminal echo")?;
    let line = tokio::select! {
        biased;
        () = cancel.cancelled() => None,
        line = read_line(&tty) => Some(line),
    };
    drop(echo);
    let Some(line) = line else {
        writeln!(out)?;
        bail!(CANCELLED);
    };
    let key = line?.trim().to_owned();
    if key.is_empty() {
        bail!("no API key entered");
    }
    Ok(key)
}

async fn read_line<T: AsFd + AsRawFd>(tty: &AsyncFd<T>) -> Result<String> {
    let mut line = Vec::new();
    let mut chunk = [0_u8; CHUNK];
    loop {
        let mut ready = tty.readable().await?;
        let read = rustix::io::read(tty.get_ref(), &mut chunk)
            .context("could not read the API key from the terminal")?;
        ready.clear_ready();
        line.extend_from_slice(chunk.get(..read).unwrap_or_default());
        if read == 0 || line.contains(&b'\n') {
            break;
        }
    }
    String::from_utf8(line).context("the API key is not valid UTF-8")
}

#[cfg(test)]
#[path = "prompt_tests.rs"]
mod tests;
