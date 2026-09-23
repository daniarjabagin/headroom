use std::collections::BTreeMap;
use std::collections::btree_map::Entry;
use std::path::{Path, PathBuf};

use headroom_core::cursor::LogCursors;
use headroom_core::event::{EventKey, UsageEvent};
use headroom_core::provider::ProviderError;

use super::config::SESSIONS_DIR;
use super::turn::parse_line;
use crate::jsonl;

const UPDATES_FILE: &str = "updates.jsonl";

pub(super) fn read_usage(
    home: &Path,
    cursors: &mut LogCursors,
) -> Result<Vec<UsageEvent>, ProviderError> {
    jsonl::prune_missing(cursors);
    let mut events = LargestByKey::default();
    for path in update_files(home)? {
        let found = jsonl::read_new_lines_or_skip(&path, cursors, |_| Vec::new(), parse_into);
        for event in found.into_iter().flatten() {
            events.insert(event);
        }
    }
    Ok(events.into_sorted())
}

fn update_files(home: &Path) -> Result<Vec<PathBuf>, ProviderError> {
    let files = jsonl::jsonl_files(&home.join(SESSIONS_DIR))?;
    Ok(files
        .into_iter()
        .filter(|path| path.file_name().is_some_and(|name| name == UPDATES_FILE))
        .collect())
}

fn parse_into(found: &mut Vec<UsageEvent>, line: &str) {
    found.extend(parse_line(line));
}

#[derive(Default)]
struct LargestByKey(BTreeMap<EventKey, UsageEvent>);

impl LargestByKey {
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
