use std::io;
use std::path::{Path, PathBuf};

use headroom_core::cursor::{FileCursor, LogCursors};
use headroom_core::event::UsageEvent;
use headroom_core::provider::ProviderError;
use jiff::Timestamp;

use super::usage_parser::ParserState;
use crate::jsonl::{self, JsonlError};

const LOG_DIRS: [&str; 2] = ["sessions", "archived_sessions"];

pub(super) fn read_usage(
    home: &Path,
    cursors: &mut LogCursors,
    now: Timestamp,
) -> Result<Vec<UsageEvent>, ProviderError> {
    jsonl::prune_missing(cursors);
    let mut events = Vec::new();
    for path in rollout_files(home)? {
        read_file(&path, cursors.cursor_mut(&path), now, &mut events)?;
    }
    Ok(events)
}

pub(super) fn rollout_files(home: &Path) -> Result<Vec<PathBuf>, JsonlError> {
    let mut files = Vec::new();
    for dir in LOG_DIRS {
        files.extend(jsonl::jsonl_files(&home.join(dir))?);
    }
    Ok(files)
}

fn read_file(
    path: &Path,
    cursor: &mut FileCursor,
    now: Timestamp,
    events: &mut Vec<UsageEvent>,
) -> Result<(), ProviderError> {
    let lines = match jsonl::read_new_lines(path, cursor) {
        Ok(lines) => lines,
        Err(error) if is_not_found(&error) => return Ok(()),
        Err(error) => return Err(error.into()),
    };
    let mut state = ParserState::from_value(&cursor.state);
    for line in &lines {
        state.consume(line, events);
    }
    state.settle(now, events);
    cursor.state = state
        .to_value()
        .map_err(|error| ProviderError::LocalData(format!("cannot store parser state: {error}")))?;
    Ok(())
}

fn is_not_found(error: &JsonlError) -> bool {
    match error {
        JsonlError::Io { source, .. } => source.kind() == io::ErrorKind::NotFound,
    }
}

#[cfg(test)]
#[path = "local_usage_tests.rs"]
mod tests;
