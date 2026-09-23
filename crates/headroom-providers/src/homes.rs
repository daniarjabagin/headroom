use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

pub(crate) fn canonical(path: &Path) -> PathBuf {
    fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

pub(crate) fn unique_dirs(dirs: impl IntoIterator<Item = PathBuf>) -> Vec<PathBuf> {
    let mut seen = BTreeSet::new();
    dirs.into_iter()
        .filter(|dir| seen.insert(canonical(dir)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duplicates_are_dropped_by_canonical_path() {
        let root = tempfile::tempdir().unwrap();
        let real = root.path().join("real");
        let link = root.path().join("link");
        let other = root.path().join("other");
        fs::create_dir_all(&real).unwrap();
        fs::create_dir_all(&other).unwrap();
        std::os::unix::fs::symlink(&real, &link).unwrap();
        let dirs = unique_dirs([link.clone(), other.clone(), real.clone(), link.clone()]);
        assert_eq!(dirs, [link, other]);
    }

    #[test]
    fn missing_dirs_compare_by_their_path() {
        let dirs = unique_dirs([PathBuf::from("/nowhere/a"), PathBuf::from("/nowhere/a")]);
        assert_eq!(dirs, [PathBuf::from("/nowhere/a")]);
    }
}
