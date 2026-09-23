use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::{JsonlError, io_error};

pub fn jsonl_files(root: &Path) -> Result<Vec<PathBuf>, JsonlError> {
    match fs::metadata(root) {
        Ok(metadata) if metadata.is_dir() => collect(root),
        Ok(_) => Ok(Vec::new()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(error) => Err(io_error(root)(error)),
    }
}

fn collect(root: &Path) -> Result<Vec<PathBuf>, JsonlError> {
    let mut files = Vec::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(dir) = pending.pop() {
        for (path, kind) in entries(&dir)? {
            if kind.is_dir() {
                pending.push(path);
            } else if kind.is_file() && is_jsonl(&path) {
                files.push(path);
            }
        }
    }
    files.sort();
    Ok(files)
}

fn entries(dir: &Path) -> Result<Vec<(PathBuf, fs::FileType)>, JsonlError> {
    let reader = match fs::read_dir(dir) {
        Ok(reader) => reader,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(io_error(dir)(error)),
    };
    reader
        .map(|entry| {
            let entry = entry.map_err(io_error(dir))?;
            let kind = entry.file_type().map_err(io_error(&entry.path()))?;
            Ok((entry.path(), kind))
        })
        .collect()
}

fn is_jsonl(path: &Path) -> bool {
    path.extension()
        .is_some_and(|extension| extension == "jsonl")
}

#[cfg(test)]
mod tests {
    use std::os::unix::fs::symlink;

    use super::*;

    fn touch(path: &Path) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, "{}\n").unwrap();
    }

    #[test]
    fn finds_nested_jsonl_files_sorted() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        touch(&root.join("b/2026/09/rollout-2.jsonl"));
        touch(&root.join("a.jsonl"));
        touch(&root.join("b/2026/09/rollout-1.jsonl"));
        touch(&root.join("b/notes.json"));
        touch(&root.join("b/jsonl"));
        let found = jsonl_files(root).unwrap();
        assert_eq!(
            found,
            [
                root.join("a.jsonl"),
                root.join("b/2026/09/rollout-1.jsonl"),
                root.join("b/2026/09/rollout-2.jsonl"),
            ]
        );
    }

    #[test]
    fn does_not_follow_symlinks() {
        let dir = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        touch(&outside.path().join("elsewhere.jsonl"));
        let root = dir.path().join("root");
        touch(&root.join("real.jsonl"));
        symlink(outside.path(), root.join("linked-dir")).unwrap();
        symlink(
            outside.path().join("elsewhere.jsonl"),
            root.join("linked.jsonl"),
        )
        .unwrap();
        assert_eq!(jsonl_files(&root).unwrap(), [root.join("real.jsonl")]);
    }

    #[test]
    fn symlinked_root_is_listed() {
        let dir = tempfile::tempdir().unwrap();
        touch(&dir.path().join("real/x.jsonl"));
        let link = dir.path().join("link");
        symlink(dir.path().join("real"), &link).unwrap();
        assert_eq!(jsonl_files(&link).unwrap(), [link.join("x.jsonl")]);
    }

    #[test]
    fn missing_or_file_root_is_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(jsonl_files(&dir.path().join("absent")).unwrap().is_empty());
        let file = dir.path().join("file.jsonl");
        touch(&file);
        assert!(jsonl_files(&file).unwrap().is_empty());
    }
}
