use std::io::{self, Write};

pub struct LossyWriter<W: Write> {
    out: W,
    open: bool,
}

impl<W: Write> LossyWriter<W> {
    pub fn new(out: W) -> LossyWriter<W> {
        LossyWriter { out, open: true }
    }
}

impl<W: Write> Write for LossyWriter<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if self.open && self.out.write_all(buf).is_err() {
            self.open = false;
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        if self.open && self.out.flush().is_err() {
            self.open = false;
        }
        Ok(())
    }
}

#[cfg(test)]
#[derive(Default)]
pub struct ClosedPipe {
    attempts: usize,
}

#[cfg(test)]
impl Write for ClosedPipe {
    fn write(&mut self, _: &[u8]) -> io::Result<usize> {
        self.attempts += 1;
        Err(io::Error::from(io::ErrorKind::BrokenPipe))
    }

    fn flush(&mut self) -> io::Result<()> {
        Err(io::Error::from(io::ErrorKind::BrokenPipe))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_pass_through_while_the_reader_is_there() {
        let mut out = LossyWriter::new(Vec::new());
        writeln!(out, "step one").unwrap();
        out.flush().unwrap();
        assert_eq!(out.out, b"step one\n");
    }

    #[test]
    fn a_closed_reader_is_dropped_after_the_first_failure() {
        let mut out = LossyWriter::new(ClosedPipe::default());
        writeln!(out, "step one").unwrap();
        out.flush().unwrap();
        writeln!(out, "step two").unwrap();
        assert!(!out.open);
        assert_eq!(out.out.attempts, 1);
    }
}
