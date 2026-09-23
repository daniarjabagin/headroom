use std::collections::BTreeMap;
use std::collections::btree_map::Entry;
use std::io;
use std::path::Path;

use headroom_core::cursor::LogCursors;
use headroom_core::event::{EventKey, UsageEvent};
use headroom_core::provider::ProviderError;

use super::log_record::parse_line;
use crate::jsonl::{self, JsonlError};

const PROJECTS_DIR: &str = "projects";

pub(super) fn read_usage(
    home: &Path,
    cursors: &mut LogCursors,
) -> Result<Vec<UsageEvent>, ProviderError> {
    jsonl::prune_missing(cursors);
    let mut events = LatestByKey::default();
    for path in jsonl::jsonl_files(&home.join(PROJECTS_DIR))? {
        for event in new_lines(&path, cursors)?
            .iter()
            .filter_map(|line| parse_line(line))
        {
            events.insert(event);
        }
    }
    Ok(events.into_sorted())
}

fn new_lines(path: &Path, cursors: &mut LogCursors) -> Result<Vec<String>, ProviderError> {
    match jsonl::read_new_lines(path, cursors.cursor_mut(path)) {
        Ok(lines) => Ok(lines),
        Err(JsonlError::Io { source, .. }) if source.kind() == io::ErrorKind::NotFound => {
            cursors.retain(|known| known != path);
            Ok(Vec::new())
        }
        Err(error) => Err(error.into()),
    }
}

#[derive(Default)]
struct LatestByKey(BTreeMap<EventKey, UsageEvent>);

impl LatestByKey {
    fn insert(&mut self, event: UsageEvent) {
        match self.0.entry(event.key.clone()) {
            Entry::Vacant(slot) => {
                slot.insert(event);
            }
            Entry::Occupied(mut slot) => {
                if event.tokens.total() > slot.get().tokens.total() {
                    slot.insert(event);
                }
            }
        }
    }

    fn into_sorted(self) -> Vec<UsageEvent> {
        let mut events: Vec<UsageEvent> = self.0.into_values().collect();
        events.sort_by(|a, b| a.at.cmp(&b.at).then_with(|| a.key.cmp(&b.key)));
        events
    }
}

#[cfg(test)]
#[path = "local_usage_tests.rs"]
mod tests;
