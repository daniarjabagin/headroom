use std::fs::{self, File, Metadata};
use std::io::{self, BufRead, BufReader, Read, Seek, SeekFrom};
use std::os::unix::fs::MetadataExt;
use std::path::Path;

use headroom_core::cursor::{FileCursor, LogCursors};

use super::{JsonlError, io_error, warn_skipped};

const READ_BUFFER: usize = 64 * 1024;

pub fn read_new_lines<S>(
    path: &Path,
    cursor: &mut FileCursor,
    init: impl FnOnce(&FileCursor) -> S,
    visit: impl FnMut(&mut S, &str),
) -> Result<S, JsonlError> {
    let mut file = File::open(path).map_err(io_error(path))?;
    let stamp = FileStamp::of(&file.metadata().map_err(io_error(path))?);
    if stamp.invalidates(cursor) {
        *cursor = FileCursor::default();
    }
    let mut acc = init(cursor);
    file.seek(SeekFrom::Start(cursor.offset))
        .map_err(io_error(path))?;
    let range = file.take(stamp.size.saturating_sub(cursor.offset));
    let consumed = visit_complete_lines(range, &mut acc, visit).map_err(io_error(path))?;
    cursor.offset += consumed;
    stamp.store(cursor);
    Ok(acc)
}

pub fn read_new_lines_or_skip<S>(
    path: &Path,
    cursors: &mut LogCursors,
    init: impl FnOnce(&FileCursor) -> S,
    visit: impl FnMut(&mut S, &str),
) -> Option<S> {
    let previous = cursors.0.get(path).cloned();
    match read_new_lines(path, cursors.cursor_mut(path), init, visit) {
        Ok(acc) => Some(acc),
        Err(JsonlError::Io { source, .. }) => {
            restore(cursors, path, previous);
            warn_skipped(path, &source);
            None
        }
    }
}

fn visit_complete_lines<S>(
    range: impl Read,
    acc: &mut S,
    mut visit: impl FnMut(&mut S, &str),
) -> io::Result<u64> {
    let mut reader = BufReader::with_capacity(READ_BUFFER, range);
    let mut line = Vec::new();
    let mut consumed = 0;
    loop {
        line.clear();
        let read = reader.read_until(b'\n', &mut line)?;
        if read == 0 || line.last() != Some(&b'\n') {
            return Ok(consumed);
        }
        consumed += read as u64;
        let text = trim_line_end(&line);
        if !text.iter().all(u8::is_ascii_whitespace) {
            visit(acc, &String::from_utf8_lossy(text));
        }
    }
}

fn trim_line_end(line: &[u8]) -> &[u8] {
    let line = line.strip_suffix(b"\n").unwrap_or(line);
    line.strip_suffix(b"\r").unwrap_or(line)
}

fn restore(cursors: &mut LogCursors, path: &Path, previous: Option<FileCursor>) {
    match previous {
        Some(cursor) => *cursors.cursor_mut(path) = cursor,
        None => cursors.retain(|known| known != path),
    }
}

pub fn prune_missing(cursors: &mut LogCursors) {
    cursors.retain(|path| !is_missing(path));
}

fn is_missing(path: &Path) -> bool {
    matches!(fs::symlink_metadata(path), Err(error) if error.kind() == io::ErrorKind::NotFound)
}

#[derive(Debug, Clone, Copy)]
struct FileStamp {
    inode: u64,
    size: u64,
    mtime_ns: i128,
}

impl FileStamp {
    const NANOS_PER_SECOND: i128 = 1_000_000_000;

    fn of(metadata: &Metadata) -> FileStamp {
        FileStamp {
            inode: metadata.ino(),
            size: metadata.len(),
            mtime_ns: i128::from(metadata.mtime()) * Self::NANOS_PER_SECOND
                + i128::from(metadata.mtime_nsec()),
        }
    }

    fn invalidates(self, cursor: &FileCursor) -> bool {
        let replaced = self.inode != cursor.inode;
        let shrunk = self.size < cursor.size || self.size < cursor.offset;
        let rewritten = self.mtime_ns < cursor.mtime_ns
            || (self.size == cursor.size && self.mtime_ns != cursor.mtime_ns);
        replaced || shrunk || rewritten
    }

    fn store(self, cursor: &mut FileCursor) {
        cursor.inode = self.inode;
        cursor.size = self.size;
        cursor.mtime_ns = self.mtime_ns;
    }
}

#[cfg(test)]
#[path = "read_tests.rs"]
mod tests;
