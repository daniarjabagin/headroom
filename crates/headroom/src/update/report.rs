use std::io::{self, Write};

use anyhow::Result;
use serde::Serialize;

use crate::accounts::progress::JsonLines;

const INSTALLER_STEP: &str = "==> ";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum UpdateEvent {
    Step { text: String },
    Done { version: String, relogin: bool },
    Error { message: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finished {
    pub version: String,
    pub relogin: bool,
    pub message: String,
}

pub enum Reporter<W: Write> {
    Text(W),
    Json(JsonLines<W>),
}

impl<W: Write> Reporter<W> {
    pub fn step(&mut self, text: &str) -> Result<()> {
        match self {
            Reporter::Text(out) => writeln!(out, "{text}")?,
            Reporter::Json(lines) => lines.emit(&UpdateEvent::Step {
                text: text.to_owned(),
            })?,
        }
        Ok(())
    }

    pub fn installer_line(&mut self, line: &str) -> Result<()> {
        match self {
            Reporter::Text(out) => writeln!(out, "{line}")?,
            Reporter::Json(_) => match line.strip_prefix(INSTALLER_STEP) {
                Some(step) => self.step(step)?,
                None if line.trim().is_empty() => {}
                None => writeln!(io::stderr(), "{line}")?,
            },
        }
        Ok(())
    }

    pub fn finish(&mut self, outcome: Result<Finished>) -> Result<()> {
        match (self, outcome) {
            (Reporter::Text(out), Ok(finished)) => writeln!(out, "{}", finished.message)?,
            (Reporter::Text(_), Err(error)) => return Err(error),
            (Reporter::Json(lines), Ok(finished)) => lines.emit(&UpdateEvent::Done {
                version: finished.version,
                relogin: finished.relogin,
            })?,
            (Reporter::Json(lines), Err(error)) => {
                lines.emit(&UpdateEvent::Error {
                    message: format!("{error:#}"),
                })?;
                return Err(error);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn finished() -> Finished {
        Finished {
            version: "0.5.0".into(),
            relogin: true,
            message: "Updated to 0.5.0.".into(),
        }
    }

    fn written(reporter: Reporter<Vec<u8>>) -> String {
        match reporter {
            Reporter::Json(lines) => String::from_utf8(lines.into_inner()).unwrap(),
            Reporter::Text(out) => String::from_utf8(out).unwrap(),
        }
    }

    #[test]
    fn json_progress_is_one_event_per_line() {
        let mut reporter = Reporter::Json(JsonLines::new(Vec::new()));
        reporter.step("Downloading 0.5.0…").unwrap();
        reporter
            .installer_line("==> Installing ~/.local/bin/headroom")
            .unwrap();
        reporter.installer_line("").unwrap();
        reporter.finish(Ok(finished())).unwrap();
        assert_eq!(
            written(reporter),
            "{\"event\":\"step\",\"text\":\"Downloading 0.5.0…\"}\n\
             {\"event\":\"step\",\"text\":\"Installing ~/.local/bin/headroom\"}\n\
             {\"event\":\"done\",\"version\":\"0.5.0\",\"relogin\":true}\n"
        );
    }

    #[test]
    fn failures_end_with_an_error_event() {
        let mut reporter = Reporter::Json(JsonLines::new(Vec::new()));
        let failed = reporter.finish(Err(anyhow::anyhow!("checksum mismatch")));
        assert!(failed.is_err());
        assert_eq!(
            written(reporter),
            "{\"event\":\"error\",\"message\":\"checksum mismatch\"}\n"
        );
    }

    #[test]
    fn text_progress_prints_plain_lines() {
        let mut reporter = Reporter::Text(Vec::new());
        reporter.step("Verifying the checksum…").unwrap();
        reporter.installer_line("==> Installing").unwrap();
        reporter.finish(Ok(finished())).unwrap();
        assert_eq!(
            written(reporter),
            "Verifying the checksum…\n==> Installing\nUpdated to 0.5.0.\n"
        );
    }
}
