use std::fs::{self, File, Metadata};
use std::io::{self, Read, Seek, SeekFrom};
use std::os::unix::fs::MetadataExt;
use std::path::Path;

use headroom_core::cursor::{FileCursor, LogCursors};

use super::{JsonlError, io_error};

pub fn read_new_lines(path: &Path, cursor: &mut FileCursor) -> Result<Vec<String>, JsonlError> {
    let mut file = File::open(path).map_err(io_error(path))?;
    let stamp = FileStamp::of(&file.metadata().map_err(io_error(path))?);
    if stamp.invalidates(cursor) {
        *cursor = FileCursor::default();
    }
    let chunk = read_range(&mut file, cursor.offset, stamp.size).map_err(io_error(path))?;
    let complete = complete_prefix(&chunk);
    cursor.offset += complete.len() as u64;
    stamp.store(cursor);
    Ok(split_lines(complete))
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

fn read_range(file: &mut File, offset: u64, size: u64) -> io::Result<Vec<u8>> {
    file.seek(SeekFrom::Start(offset))?;
    let mut chunk = Vec::new();
    file.take(size.saturating_sub(offset))
        .read_to_end(&mut chunk)?;
    Ok(chunk)
}

fn complete_prefix(chunk: &[u8]) -> &[u8] {
    let end = chunk
        .iter()
        .rposition(|&byte| byte == b'\n')
        .map_or(0, |index| index + 1);
    &chunk[..end]
}

fn split_lines(bytes: &[u8]) -> Vec<String> {
    bytes
        .split(|&byte| byte == b'\n')
        .map(|line| line.strip_suffix(b"\r").unwrap_or(line))
        .filter(|line| !line.iter().all(u8::is_ascii_whitespace))
        .map(|line| String::from_utf8_lossy(line).into_owned())
        .collect()
}

#[cfg(test)]
#[path = "read_tests.rs"]
mod tests;
