use std::fmt;
use std::io::{self, Write};

use anyhow::Result;

#[derive(Debug)]
pub struct OutputClosed;

impl fmt::Display for OutputClosed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("standard output was closed")
    }
}

impl std::error::Error for OutputClosed {}

pub fn print(text: &str) -> Result<()> {
    write_to(&mut io::stdout().lock(), text)
}

pub fn print_line(text: &str) -> Result<()> {
    print(&format!("{text}\n"))
}

pub fn is_closed_output(error: &anyhow::Error) -> bool {
    error.downcast_ref::<OutputClosed>().is_some()
}

fn write_to(out: &mut impl Write, text: &str) -> Result<()> {
    out.write_all(text.as_bytes())
        .and_then(|()| out.flush())
        .map_err(closed_or_failed)
}

fn closed_or_failed(error: io::Error) -> anyhow::Error {
    if error.kind() == io::ErrorKind::BrokenPipe {
        anyhow::Error::new(OutputClosed)
    } else {
        anyhow::Error::new(error)
    }
}

#[cfg(test)]
#[path = "output_tests.rs"]
mod tests;
