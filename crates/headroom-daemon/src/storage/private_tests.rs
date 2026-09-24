use std::fs;
use std::os::unix::fs::PermissionsExt;

use rustix::fs::Mode;
use rustix::process::umask;

use super::*;
use crate::settings::Settings;
use crate::storage::{Storage, settings};

struct PermissiveUmask(Mode);

impl PermissiveUmask {
    fn set() -> PermissiveUmask {
        PermissiveUmask(umask(Mode::empty()))
    }
}

impl Drop for PermissiveUmask {
    fn drop(&mut self) {
        umask(self.0);
    }
}

fn mode(path: &Path) -> u32 {
    fs::metadata(path).unwrap().permissions().mode() & 0o777
}

fn write_something(storage: &Storage) {
    storage
        .blocking(|conn| settings::save(conn, &Settings::default()))
        .unwrap();
}

#[test]
fn a_new_database_is_private_even_with_a_permissive_umask() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("state/headroom/headroom.db");
    let storage = {
        let _umask = PermissiveUmask::set();
        let storage = Storage::open(&path).unwrap();
        write_something(&storage);
        storage
    };
    assert_eq!(mode(&root.path().join("state")), 0o700);
    assert_eq!(mode(path.parent().unwrap()), 0o700);
    assert_eq!(mode(&path), 0o600);
    for side in side_files(&path) {
        assert_eq!(mode(&side), 0o600, "{}", side.display());
    }
    drop(storage);
}

fn loose_install(dir: &Path) -> PathBuf {
    fs::create_dir_all(dir).unwrap();
    fs::set_permissions(dir, Permissions::from_mode(0o755)).unwrap();
    let path = dir.join("headroom.db");
    for file in std::iter::once(path.clone()).chain(side_files(&path)) {
        fs::write(&file, "").unwrap();
        fs::set_permissions(&file, Permissions::from_mode(0o644)).unwrap();
    }
    path
}

#[test]
fn an_existing_install_is_tightened_on_open() {
    let root = tempfile::tempdir().unwrap();
    let dir = root.path().join("headroom");
    let path = loose_install(&dir);
    prepare(&path, Some(&dir)).unwrap();
    assert_eq!(mode(&dir), 0o700);
    for file in std::iter::once(path.clone()).chain(side_files(&path)) {
        assert_eq!(mode(&file), 0o600, "{}", file.display());
    }
}

#[test]
fn a_directory_headroom_does_not_own_keeps_its_mode() {
    let root = tempfile::tempdir().unwrap();
    let dir = root.path().join("shared");
    let path = loose_install(&dir);
    prepare(&path, Some(root.path())).unwrap();
    assert_eq!(mode(&dir), 0o755);
    assert_eq!(mode(&path), 0o600);
}
