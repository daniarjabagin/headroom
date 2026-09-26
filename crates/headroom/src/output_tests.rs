use anyhow::Context;

use super::*;

struct Failing(io::ErrorKind);

impl Write for Failing {
    fn write(&mut self, _: &[u8]) -> io::Result<usize> {
        Err(io::Error::from(self.0))
    }

    fn flush(&mut self) -> io::Result<()> {
        Err(io::Error::from(self.0))
    }
}

struct Unflushable(Vec<u8>);

impl Write for Unflushable {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Err(io::Error::from(io::ErrorKind::BrokenPipe))
    }
}

#[test]
fn a_write_to_a_closed_pipe_counts_as_closed_output() {
    let error = write_to(&mut Failing(io::ErrorKind::BrokenPipe), "{}\n").unwrap_err();
    assert!(is_closed_output(&error));
}

#[test]
fn a_flush_to_a_closed_pipe_counts_as_closed_output() {
    let mut out = Unflushable(Vec::new());
    let error = write_to(&mut out, "line\n").unwrap_err();
    assert!(is_closed_output(&error));
    assert_eq!(out.0, b"line\n");
}

#[test]
fn closed_output_is_found_under_added_context() {
    let error = write_to(&mut Failing(io::ErrorKind::BrokenPipe), "{}\n")
        .context("cannot print the status")
        .unwrap_err();
    assert!(is_closed_output(&error));
}

#[test]
fn other_failures_are_still_reported() {
    let denied = write_to(&mut Failing(io::ErrorKind::PermissionDenied), "{}\n").unwrap_err();
    assert!(!is_closed_output(&denied));
    let socket = anyhow::Error::from(io::Error::from(io::ErrorKind::BrokenPipe));
    assert!(!is_closed_output(&socket));
}

#[test]
fn text_reaches_an_open_writer_unchanged() {
    let mut out = Vec::new();
    write_to(&mut out, "Headroom 0.6.1\n").unwrap();
    assert_eq!(out, b"Headroom 0.6.1\n");
}
