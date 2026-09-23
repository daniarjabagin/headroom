use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct FileCursor {
    pub inode: u64,
    pub size: u64,
    pub mtime_ns: i128,
    pub offset: u64,
    pub state: serde_json::Value,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct LogCursors(pub BTreeMap<PathBuf, FileCursor>);

impl LogCursors {
    pub fn cursor_mut(&mut self, path: &Path) -> &mut FileCursor {
        self.0.entry(path.to_path_buf()).or_default()
    }

    pub fn retain(&mut self, mut keep: impl FnMut(&Path) -> bool) {
        self.0.retain(|path, _| keep(path));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cursor_mut_creates_default_once() {
        let mut cursors = LogCursors::default();
        cursors.cursor_mut(Path::new("/a.jsonl")).offset = 7;
        assert_eq!(cursors.cursor_mut(Path::new("/a.jsonl")).offset, 7);
        assert_eq!(cursors.0.len(), 1);
    }

    #[test]
    fn retain_drops_rejected_paths() {
        let mut cursors = LogCursors::default();
        cursors.cursor_mut(Path::new("/a.jsonl"));
        cursors.cursor_mut(Path::new("/b.jsonl"));
        cursors.retain(|path| path.ends_with("b.jsonl"));
        assert_eq!(
            cursors.0.keys().collect::<Vec<_>>(),
            [Path::new("/b.jsonl")]
        );
    }

    #[test]
    fn cursors_round_trip_through_json() {
        let mut cursors = LogCursors::default();
        let cursor = cursors.cursor_mut(Path::new("/s/r.jsonl"));
        cursor.inode = 42;
        cursor.mtime_ns = 1_790_000_000_123_456_789;
        cursor.state = serde_json::json!({ "model": "gpt-5.5" });
        let json = serde_json::to_string(&cursors).unwrap();
        assert_eq!(serde_json::from_str::<LogCursors>(&json).unwrap(), cursors);
    }
}
