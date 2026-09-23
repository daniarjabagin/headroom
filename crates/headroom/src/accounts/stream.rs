use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Child, ChildStdin, Command, ExitStatus, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread;
use std::time::Duration;

use anyhow::{Context, Result};

use super::login::not_started;

const QUIET_FLUSH: Duration = Duration::from_millis(100);
const DRAIN_GRACE: Duration = Duration::from_millis(200);
const CHUNK_BYTES: usize = 4_096;

pub type LineSink<'a> = dyn FnMut(&str) -> Result<()> + 'a;

pub fn run_streamed(
    mut command: Command,
    input: Box<dyn Read + Send>,
    on_line: &mut LineSink<'_>,
) -> Result<ExitStatus> {
    command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command.spawn().with_context(|| not_started(&command))?;
    let chunks = spawn_readers(&mut child);
    if let Some(stdin) = child.stdin.take() {
        thread::spawn(move || forward_input(input, stdin));
    }
    let mut lines = Lines::default();
    match pump(&mut child, &chunks, &mut lines, on_line) {
        Ok(status) => Ok(status),
        Err(error) => {
            stop(&mut child);
            Err(error)
        }
    }
}

struct Chunk {
    stream: usize,
    bytes: Vec<u8>,
}

#[derive(Default)]
struct Lines {
    pending: [Vec<u8>; 2],
}

impl Lines {
    fn push(&mut self, chunk: &Chunk, on_line: &mut LineSink<'_>) -> Result<()> {
        let Some(buffer) = self.pending.get_mut(chunk.stream) else {
            return Ok(());
        };
        buffer.extend_from_slice(&chunk.bytes);
        while let Some(end) = buffer.iter().position(|byte| *byte == b'\n') {
            let line: Vec<u8> = buffer.drain(..=end).collect();
            emit(line.strip_suffix(b"\n").unwrap_or(&line), on_line)?;
        }
        Ok(())
    }

    fn flush(&mut self, on_line: &mut LineSink<'_>) -> Result<()> {
        for buffer in &mut self.pending {
            if !buffer.is_empty() {
                emit(&std::mem::take(buffer), on_line)?;
            }
        }
        Ok(())
    }
}

fn emit(bytes: &[u8], on_line: &mut LineSink<'_>) -> Result<()> {
    let text = String::from_utf8_lossy(bytes);
    on_line(text.trim_end_matches('\r'))
}

fn spawn_readers(child: &mut Child) -> Receiver<Chunk> {
    let (sender, receiver) = mpsc::channel();
    if let Some(stdout) = child.stdout.take() {
        spawn_reader(0, stdout, sender.clone());
    }
    if let Some(stderr) = child.stderr.take() {
        spawn_reader(1, stderr, sender);
    }
    receiver
}

fn spawn_reader(stream: usize, mut pipe: impl Read + Send + 'static, sender: Sender<Chunk>) {
    thread::spawn(move || {
        let mut buffer = [0_u8; CHUNK_BYTES];
        while let Ok(read) = pipe.read(&mut buffer) {
            let Some(bytes) = buffer.get(..read).filter(|bytes| !bytes.is_empty()) else {
                return;
            };
            let chunk = Chunk {
                stream,
                bytes: bytes.to_vec(),
            };
            if sender.send(chunk).is_err() {
                return;
            }
        }
    });
}

fn forward_input(input: Box<dyn Read + Send>, mut stdin: ChildStdin) {
    let mut reader = BufReader::new(input);
    let mut line = String::new();
    loop {
        line.clear();
        match reader.read_line(&mut line) {
            Ok(0) | Err(_) => return,
            Ok(_) => {
                if stdin
                    .write_all(line.as_bytes())
                    .and_then(|()| stdin.flush())
                    .is_err()
                {
                    return;
                }
            }
        }
    }
}

fn pump(
    child: &mut Child,
    chunks: &Receiver<Chunk>,
    lines: &mut Lines,
    on_line: &mut LineSink<'_>,
) -> Result<ExitStatus> {
    loop {
        match chunks.recv_timeout(QUIET_FLUSH) {
            Ok(chunk) => lines.push(&chunk, on_line)?,
            Err(RecvTimeoutError::Disconnected) => {
                lines.flush(on_line)?;
                return Ok(child.wait()?);
            }
            Err(RecvTimeoutError::Timeout) => {
                lines.flush(on_line)?;
                if let Some(status) = child.try_wait().context("could not wait for the login")? {
                    drain(chunks, lines, on_line)?;
                    return Ok(status);
                }
            }
        }
    }
}

fn drain(chunks: &Receiver<Chunk>, lines: &mut Lines, on_line: &mut LineSink<'_>) -> Result<()> {
    while let Ok(chunk) = chunks.recv_timeout(DRAIN_GRACE) {
        lines.push(&chunk, on_line)?;
    }
    lines.flush(on_line)
}

fn stop(child: &mut Child) {
    if let Err(error) = child.kill() {
        tracing::debug!(%error, "login process already gone");
    }
    if let Err(error) = child.wait() {
        tracing::debug!(%error, "could not reap the login process");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn collect(chunks: &[(usize, &str)]) -> Vec<String> {
        let mut lines = Lines::default();
        let mut seen = Vec::new();
        let mut sink = |line: &str| {
            seen.push(line.to_owned());
            Ok(())
        };
        for (stream, text) in chunks {
            let chunk = Chunk {
                stream: *stream,
                bytes: text.as_bytes().to_vec(),
            };
            lines.push(&chunk, &mut sink).unwrap();
        }
        lines.flush(&mut sink).unwrap();
        seen
    }

    #[test]
    fn lines_are_assembled_per_stream() {
        let seen = collect(&[
            (0, "hel"),
            (1, "warn"),
            (0, "lo\r\nwor"),
            (1, "ing\n"),
            (0, "ld\n\nPaste code: "),
        ]);
        assert_eq!(seen, ["hello", "warning", "world", "", "Paste code: "]);
    }

    #[test]
    fn prompts_without_a_newline_arrive_before_input_is_needed() {
        let (reader, writer) = std::io::pipe().unwrap();
        let mut answer = Some(writer);
        let mut command = Command::new("sh");
        command.args(["-c", "printf 'Code: '; read code; echo \"got $code\""]);
        let mut seen = Vec::new();
        let mut sink = |line: &str| {
            if line == "Code: "
                && let Some(mut pipe) = answer.take()
            {
                pipe.write_all(b"42\n").unwrap();
            }
            seen.push(line.to_owned());
            Ok(())
        };
        let status = run_streamed(command, Box::new(reader), &mut sink).unwrap();
        assert!(status.success());
        assert_eq!(seen, ["Code: ", "got 42"]);
    }
}
