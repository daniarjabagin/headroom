use std::io::Write;

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

pub enum Progress<W: Write> {
    Text(W),
    Json(JsonLines<W>),
}

pub struct Reporter<W: Write, E: Write> {
    progress: Progress<W>,
    diagnostics: E,
}

impl<W: Write, E: Write> Reporter<W, E> {
    pub fn text(out: W, diagnostics: E) -> Reporter<W, E> {
        Reporter {
            progress: Progress::Text(out),
            diagnostics,
        }
    }

    pub fn json(out: W, diagnostics: E) -> Reporter<W, E> {
        Reporter {
            progress: Progress::Json(JsonLines::new(out)),
            diagnostics,
        }
    }

    #[cfg(test)]
    pub fn into_parts(self) -> (W, E) {
        match self.progress {
            Progress::Text(out) => (out, self.diagnostics),
            Progress::Json(lines) => (lines.into_inner(), self.diagnostics),
        }
    }

    pub fn step(&mut self, text: &str) -> Result<()> {
        match &mut self.progress {
            Progress::Text(out) => writeln!(out, "{text}")?,
            Progress::Json(lines) => lines.emit(&UpdateEvent::Step {
                text: text.to_owned(),
            })?,
        }
        Ok(())
    }

    pub fn installer_line(&mut self, line: &str) -> Result<()> {
        match &mut self.progress {
            Progress::Text(out) => writeln!(out, "{line}")?,
            Progress::Json(_) => match line.strip_prefix(INSTALLER_STEP) {
                Some(step) => self.step(step)?,
                None if line.trim().is_empty() => {}
                None => writeln!(self.diagnostics, "{line}")?,
            },
        }
        Ok(())
    }

    pub fn installer_diagnostics(&mut self, bytes: &[u8]) -> Result<()> {
        self.diagnostics.write_all(bytes)?;
        Ok(())
    }

    pub fn finish(&mut self, outcome: Result<Finished>) -> Result<()> {
        match (&mut self.progress, outcome) {
            (Progress::Text(out), Ok(finished)) => writeln!(out, "{}", finished.message)?,
            (Progress::Text(_), Err(error)) => return Err(error),
            (Progress::Json(lines), Ok(finished)) => lines.emit(&UpdateEvent::Done {
                version: finished.version,
                relogin: finished.relogin,
            })?,
            (Progress::Json(lines), Err(error)) => {
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

    fn written(reporter: Reporter<Vec<u8>, Vec<u8>>) -> (String, String) {
        let (out, diagnostics) = reporter.into_parts();
        (
            String::from_utf8(out).unwrap(),
            String::from_utf8(diagnostics).unwrap(),
        )
    }

    #[test]
    fn json_progress_is_one_event_per_line() {
        let mut reporter = Reporter::json(Vec::new(), Vec::new());
        reporter.step("Downloading 0.5.0…").unwrap();
        reporter
            .installer_line("==> Installing ~/.local/bin/headroom")
            .unwrap();
        reporter.installer_line("").unwrap();
        reporter.installer_line("Created symlink").unwrap();
        reporter.installer_diagnostics(b"warning: slow\n").unwrap();
        reporter.finish(Ok(finished())).unwrap();
        assert_eq!(
            written(reporter),
            (
                "{\"event\":\"step\",\"text\":\"Downloading 0.5.0…\"}\n\
                 {\"event\":\"step\",\"text\":\"Installing ~/.local/bin/headroom\"}\n\
                 {\"event\":\"done\",\"version\":\"0.5.0\",\"relogin\":true}\n"
                    .to_owned(),
                "Created symlink\nwarning: slow\n".to_owned()
            )
        );
    }

    #[test]
    fn failures_end_with_an_error_event() {
        let mut reporter = Reporter::json(Vec::new(), Vec::new());
        let failed = reporter.finish(Err(anyhow::anyhow!("checksum mismatch")));
        assert!(failed.is_err());
        assert_eq!(
            written(reporter).0,
            "{\"event\":\"error\",\"message\":\"checksum mismatch\"}\n"
        );
    }

    #[test]
    fn text_progress_prints_plain_lines() {
        let mut reporter = Reporter::text(Vec::new(), Vec::new());
        reporter.step("Verifying the checksum…").unwrap();
        reporter.installer_line("==> Installing").unwrap();
        reporter.finish(Ok(finished())).unwrap();
        assert_eq!(
            written(reporter).0,
            "Verifying the checksum…\n==> Installing\nUpdated to 0.5.0.\n"
        );
    }
}
